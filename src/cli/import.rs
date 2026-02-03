use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use rfd::FileDialog;
use serde::Deserialize;

/// Opens a native file picker dialog to select a CSV file.
/// Returns `None` if the user cancels the dialog.
pub fn pick_csv_file() -> Option<String> {
    FileDialog::new()
        .add_filter("CSV files", &["csv"])
        .set_title("Select CSV file to import")
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

use crate::db::Database;
use crate::models::{
    Address, AddressType, Email, EmailType, Organization, Person, PersonOrganization,
    PersonType, Phone, PhoneType,
};

/// A row from a CSV import file.
///
/// Headers must match field names exactly (case-sensitive: `company_name` not `COMPANY_NAME`).
/// Empty strings are converted to `None` for optional fields.
#[derive(Debug, Clone, Deserialize)]
pub struct ImportRow {
    /// Company or organization name (required)
    pub company_name: String,

    /// Street address
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub street: Option<String>,

    /// City
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub city: Option<String>,

    /// State or province
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub state: Option<String>,

    /// ZIP or postal code
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub zip_code: Option<String>,

    /// Phone number
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub phone: Option<String>,

    /// Email address
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,

    /// Website URL
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub website: Option<String>,

    /// Industry or specialty
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub industry: Option<String>,

    /// External ID (license number, vendor ID, etc.)
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub external_id: Option<String>,
}

impl ImportRow {
    /// Validate that required fields are present and non-empty.
    pub fn validate(&self) -> Result<()> {
        if self.company_name.trim().is_empty() {
            bail!("company_name is required and cannot be empty");
        }
        Ok(())
    }
}

/// Deserialize empty strings as None.
fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.trim().is_empty()))
}

/// Import results summary.
#[derive(Debug, Default)]
pub struct ImportStats {
    pub organizations: u32,
    pub contacts: u32,
    pub with_phones: u32,
    pub with_emails: u32,
    pub with_websites: u32,
    pub duplicates: u32,
    pub errors: u32,
}

/// Execute the import command.
pub fn run_import(
    db: &Database,
    file: &str,
    dry_run: bool,
    source: Option<&str>,
) -> Result<()> {
    use std::io::{stderr, Write};

    let path = Path::new(file);
    if !path.exists() {
        bail!("File not found: {}", file);
    }

    let reader = File::open(path).context("Failed to open CSV file")?;
    let mut csv_reader = csv::Reader::from_reader(reader);
    let mut stats = ImportStats::default();

    let rows: Vec<_> = csv_reader.deserialize().collect();
    let total = rows.len();

    if dry_run {
        eprintln!("Dry run: {}", file);
    } else {
        eprintln!("Importing: {}", file);
    }

    for (idx, result) in rows.into_iter().enumerate() {
        let line = idx + 2; // CSV line number (1-indexed, skip header)

        // Progress every 100 rows or at end
        if idx % 100 == 0 || idx == total - 1 {
            eprint!("\r  {}/{}", idx + 1, total);
            let _ = stderr().flush();
        }

        let row: ImportRow = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\nLine {}: parse error: {}", line, e);
                stats.errors += 1;
                continue;
            }
        };

        if let Err(e) = row.validate() {
            eprintln!("\nLine {}: validation error: {}", line, e);
            stats.errors += 1;
            continue;
        }

        match process_row(db, &row, dry_run, source) {
            Ok(None) => stats.duplicates += 1,
            Ok(Some(result)) => {
                stats.organizations += 1;
                stats.contacts += 1;
                if result.has_phone { stats.with_phones += 1; }
                if result.has_email { stats.with_emails += 1; }
                if result.has_website { stats.with_websites += 1; }
            }
            Err(e) => {
                eprintln!("\nLine {}: {}", line, e);
                stats.errors += 1;
            }
        }
    }

    eprintln!(); // Clear progress line
    print_summary(&stats, dry_run);
    Ok(())
}

fn print_summary(stats: &ImportStats, dry_run: bool) {
    let verb = if dry_run { "Would create" } else { "Created" };

    println!("\n{} {} organizations, {} contacts", verb, stats.organizations, stats.contacts);

    if stats.with_phones > 0 || stats.with_emails > 0 || stats.with_websites > 0 {
        let mut details = Vec::new();
        if stats.with_phones > 0 { details.push(format!("{} phones", stats.with_phones)); }
        if stats.with_emails > 0 { details.push(format!("{} emails", stats.with_emails)); }
        if stats.with_websites > 0 { details.push(format!("{} websites", stats.with_websites)); }
        println!("  with {}", details.join(", "));
    }

    if stats.duplicates > 0 {
        println!("Skipped {} duplicates", stats.duplicates);
    }

    if stats.errors > 0 {
        println!("Errors: {}", stats.errors);
    }
}

struct ProcessResult {
    has_phone: bool,
    has_email: bool,
    has_website: bool,
}

fn process_row(
    db: &Database,
    row: &ImportRow,
    dry_run: bool,
    _source: Option<&str>,
) -> Result<Option<ProcessResult>> {
    let name = row.company_name.trim();

    // Check for existing organization
    let existing = db.search_organizations_by_name(name, row.city.as_deref(), row.state.as_deref())?;
    if !existing.is_empty() {
        return Ok(None); // Duplicate
    }

    let result = ProcessResult {
        has_phone: row.phone.is_some(),
        has_email: row.email.is_some(),
        has_website: row.website.is_some(),
    };

    if dry_run {
        return Ok(Some(result));
    }

    // Create organization
    let mut org = Organization::new(name.to_string());
    org.city = row.city.clone();
    org.state = row.state.clone();
    org.website = row.website.clone();
    org.industry = row.industry.clone();
    org.org_type = Some("business".to_string());
    db.insert_organization(&org)?;

    // Create person (business contact placeholder)
    let mut person = Person::new();
    person.display_name = Some(name.to_string());
    person.sort_name = Some(name.to_string());
    person.search_name = Some(name.to_lowercase());
    person.person_type = PersonType::Business;
    db.insert_person(&person)?;

    // Link person to organization
    let link = PersonOrganization::new_representative(person.id, org.id);
    db.insert_person_organization(&link)?;

    // Add phone if present
    if let Some(ref phone_num) = row.phone {
        let mut phone = Phone::new(person.id, phone_num.clone());
        phone.phone_type = PhoneType::Work;
        phone.is_primary = true;
        db.insert_phone(&phone)?;
    }

    // Add email if present
    if let Some(ref email_addr) = row.email {
        let mut email = Email::new(person.id, email_addr.clone());
        email.email_type = EmailType::Work;
        email.is_primary = true;
        db.insert_email(&email)?;
    }

    // Add address if any address fields present
    if row.street.is_some() || row.city.is_some() || row.state.is_some() || row.zip_code.is_some() {
        let mut address = Address::new(person.id);
        address.street = row.street.clone();
        address.city = row.city.clone();
        address.state = row.state.clone();
        address.postal_code = row.zip_code.clone();
        address.address_type = AddressType::Work;
        address.is_primary = true;
        db.insert_address(&address)?;
    }

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_complete_row() {
        let csv_data = "\
company_name,street,city,state,zip_code,phone,email,website,industry,external_id
Acme Corp,123 Main St,Seattle,WA,98101,555-1234,info@acme.com,https://acme.com,Construction,LIC-001";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        assert_eq!(row.company_name, "Acme Corp");
        assert_eq!(row.street.as_deref(), Some("123 Main St"));
        assert_eq!(row.city.as_deref(), Some("Seattle"));
        assert_eq!(row.state.as_deref(), Some("WA"));
        assert_eq!(row.zip_code.as_deref(), Some("98101"));
        assert_eq!(row.phone.as_deref(), Some("555-1234"));
        assert_eq!(row.email.as_deref(), Some("info@acme.com"));
        assert_eq!(row.website.as_deref(), Some("https://acme.com"));
        assert_eq!(row.industry.as_deref(), Some("Construction"));
        assert_eq!(row.external_id.as_deref(), Some("LIC-001"));
        assert!(row.validate().is_ok());
    }

    #[test]
    fn parse_minimal_row() {
        let csv_data = "\
company_name,street,city,state,zip_code,phone,email,website,industry,external_id
Acme Corp,,,,,,,,,";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        assert_eq!(row.company_name, "Acme Corp");
        assert!(row.street.is_none());
        assert!(row.city.is_none());
        assert!(row.state.is_none());
        assert!(row.zip_code.is_none());
        assert!(row.phone.is_none());
        assert!(row.email.is_none());
        assert!(row.website.is_none());
        assert!(row.industry.is_none());
        assert!(row.external_id.is_none());
        assert!(row.validate().is_ok());
    }

    #[test]
    fn empty_strings_become_none() {
        let csv_data = "\
company_name,street,city,state,zip_code,phone,email,website,industry,external_id
Acme Corp,   ,  ,,,,,,,";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        // Whitespace-only strings should also become None
        assert!(row.street.is_none());
        assert!(row.city.is_none());
    }

    #[test]
    fn validate_rejects_empty_company_name() {
        let csv_data = "\
company_name,street,city,state,zip_code,phone,email,website,industry,external_id
,123 Main St,Seattle,WA,98101,555-1234,info@acme.com,https://acme.com,Construction,LIC-001";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        let err = row.validate().unwrap_err();
        assert!(err.to_string().contains("company_name"));
    }

    #[test]
    fn validate_rejects_whitespace_company_name() {
        let csv_data = "\
company_name,street,city,state,zip_code,phone,email,website,industry,external_id
   ,123 Main St,Seattle,WA,98101,555-1234,info@acme.com,https://acme.com,Construction,LIC-001";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        let err = row.validate().unwrap_err();
        assert!(err.to_string().contains("company_name"));
    }

    #[test]
    fn parse_multiple_rows() {
        let csv_data = "\
company_name,city,state
Acme Corp,Seattle,WA
Beta LLC,Portland,OR
Gamma Inc,San Francisco,CA";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let rows: Vec<ImportRow> = reader
            .deserialize()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].company_name, "Acme Corp");
        assert_eq!(rows[1].company_name, "Beta LLC");
        assert_eq!(rows[2].company_name, "Gamma Inc");
    }

    #[test]
    fn headers_with_subset_of_fields() {
        // CSV only has some columns - others should default to None
        let csv_data = "\
company_name,phone,email
Acme Corp,555-1234,info@acme.com";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        assert_eq!(row.company_name, "Acme Corp");
        assert_eq!(row.phone.as_deref(), Some("555-1234"));
        assert_eq!(row.email.as_deref(), Some("info@acme.com"));
        assert!(row.street.is_none());
        assert!(row.city.is_none());
    }

    #[test]
    fn handles_quoted_values_with_commas() {
        let csv_data = r#"company_name,city,state
"Acme, Inc.",Seattle,WA
"Beta ""The Best"" LLC",Portland,OR"#;

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let rows: Vec<ImportRow> = reader
            .deserialize()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(rows[0].company_name, "Acme, Inc.");
        assert_eq!(rows[1].company_name, r#"Beta "The Best" LLC"#);
    }

    #[test]
    fn ignores_extra_columns() {
        // CSV has columns we don't recognize - should be ignored
        let csv_data = "\
company_name,unknown_field,city,another_unknown
Acme Corp,ignored,Seattle,also_ignored";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let row: ImportRow = reader.deserialize().next().unwrap().unwrap();

        assert_eq!(row.company_name, "Acme Corp");
        assert_eq!(row.city.as_deref(), Some("Seattle"));
    }

    #[test]
    fn missing_required_column_fails_at_parse() {
        // CSV missing company_name column entirely
        let csv_data = "\
city,state
Seattle,WA";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let result: Result<ImportRow, _> = reader.deserialize().next().unwrap();

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("company_name"), "Error should mention missing field: {}", err);
    }

    #[test]
    fn handles_unicode() {
        let csv_data = "\
company_name,city
Müller GmbH,München
北京公司,北京";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let rows: Vec<ImportRow> = reader
            .deserialize()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(rows[0].company_name, "Müller GmbH");
        assert_eq!(rows[0].city.as_deref(), Some("München"));
        assert_eq!(rows[1].company_name, "北京公司");
        assert_eq!(rows[1].city.as_deref(), Some("北京"));
    }

    #[test]
    fn headers_are_case_sensitive() {
        // csv crate is case-sensitive by default - uppercase headers won't match
        let csv_data = "\
COMPANY_NAME,CITY
Acme Corp,Seattle";

        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let result: Result<ImportRow, _> = reader.deserialize().next().unwrap();

        // This SHOULD fail because headers are case-sensitive
        assert!(result.is_err(), "CSV headers are case-sensitive; COMPANY_NAME != company_name");
    }

    #[test]
    fn import_creates_org_person_and_links() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let db = crate::db::Database::open_memory().unwrap();

        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "company_name,city,state,phone,email\nAcme Corp,Seattle,WA,555-1234,info@acme.com"
        )
        .unwrap();

        run_import(&db, file.path().to_str().unwrap(), false, None).unwrap();

        // Verify organization created
        let orgs = db.search_organizations_by_name("Acme Corp", None, None).unwrap();
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].city.as_deref(), Some("Seattle"));
        assert_eq!(orgs[0].state.as_deref(), Some("WA"));

        // Verify person created and linked
        let persons = db.list_persons(100, 0).unwrap();
        assert_eq!(persons.len(), 1);
        assert_eq!(persons[0].display_name.as_deref(), Some("Acme Corp"));
        assert_eq!(persons[0].person_type, crate::models::PersonType::Business);

        // Verify phone attached
        let phones = db.get_phones_for_person(persons[0].id).unwrap();
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0].phone_number, "555-1234");

        // Verify email attached
        let emails = db.get_emails_for_person(persons[0].id).unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].email_address, "info@acme.com");
    }

    #[test]
    fn import_skips_duplicates() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let db = crate::db::Database::open_memory().unwrap();

        // Import once
        let mut file1 = NamedTempFile::new().unwrap();
        writeln!(file1, "company_name,city,state\nAcme Corp,Seattle,WA").unwrap();
        run_import(&db, file1.path().to_str().unwrap(), false, None).unwrap();

        // Import again - should skip
        let mut file2 = NamedTempFile::new().unwrap();
        writeln!(file2, "company_name,city,state\nAcme Corp,Seattle,WA").unwrap();
        run_import(&db, file2.path().to_str().unwrap(), false, None).unwrap();

        // Should still only have one organization
        let orgs = db.search_organizations_by_name("Acme Corp", None, None).unwrap();
        assert_eq!(orgs.len(), 1);

        let persons = db.list_persons(100, 0).unwrap();
        assert_eq!(persons.len(), 1);
    }

    #[test]
    fn import_dry_run_creates_nothing() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let db = crate::db::Database::open_memory().unwrap();

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "company_name,city,state\nAcme Corp,Seattle,WA").unwrap();

        run_import(&db, file.path().to_str().unwrap(), true, None).unwrap();

        // Nothing should be created
        let orgs = db.search_organizations_by_name("Acme Corp", None, None).unwrap();
        assert!(orgs.is_empty());

        let persons = db.list_persons(100, 0).unwrap();
        assert!(persons.is_empty());
    }

    #[test]
    fn import_handles_missing_file() {
        let db = crate::db::Database::open_memory().unwrap();
        let result = run_import(&db, "/nonexistent/file.csv", false, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn import_creates_address_when_fields_present() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let db = crate::db::Database::open_memory().unwrap();

        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "company_name,street,city,state,zip_code\nAcme Corp,123 Main St,Seattle,WA,98101"
        )
        .unwrap();

        run_import(&db, file.path().to_str().unwrap(), false, None).unwrap();

        let persons = db.list_persons(100, 0).unwrap();
        assert_eq!(persons.len(), 1);

        let addresses = db.get_addresses_for_person(persons[0].id).unwrap();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].street.as_deref(), Some("123 Main St"));
        assert_eq!(addresses[0].city.as_deref(), Some("Seattle"));
        assert_eq!(addresses[0].state.as_deref(), Some("WA"));
        assert_eq!(addresses[0].postal_code.as_deref(), Some("98101"));
    }
}
