use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::cli::ui::select_contact;
use crate::db::Database;
use crate::models::{Email, Person, Phone};

/// Execute the update command
pub fn run_update(
    db: &Database,
    identifier: &str,
    first: Option<String>,
    last: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    notes: Option<String>,
) -> Result<()> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Identifier cannot be empty."));
    }

    // Check if any updates provided
    if first.is_none() && last.is_none() && email.is_none() && phone.is_none() && notes.is_none() {
        return Err(anyhow!("No updates provided. Use -f, -l, -e, -p, or -n to specify changes."));
    }

    // Find the person
    let person = find_person(db, identifier)?;
    let person = match person {
        Some(p) => p,
        None => {
            println!("No contact found matching \"{}\".", identifier);
            return Ok(());
        }
    };

    // Apply updates
    let mut updated = person.clone();
    let mut changes = Vec::new();

    if let Some(ref f) = first {
        updated.name_given = Some(f.clone());
        changes.push(format!("first name -> {}", f));
    }
    if let Some(ref l) = last {
        updated.name_family = Some(l.clone());
        changes.push(format!("last name -> {}", l));
    }
    if let Some(ref n) = notes {
        updated.notes = Some(n.clone());
        changes.push("notes updated".to_string());
    }

    // Recompute display names if name changed
    if first.is_some() || last.is_some() {
        updated.compute_names();
    }

    // Update person record
    db.update_person(&updated)?;

    // Handle email update (replace primary or add new)
    if let Some(ref email_addr) = email {
        if !is_valid_email(email_addr) {
            return Err(anyhow!("Invalid email format: {}", email_addr));
        }
        update_primary_email(db, person.id, email_addr)?;
        changes.push(format!("email -> {}", email_addr));
    }

    // Handle phone update (replace primary or add new)
    if let Some(ref phone_num) = phone {
        update_primary_phone(db, person.id, phone_num)?;
        changes.push(format!("phone -> {}", phone_num));
    }

    let display_name = updated.display_name.as_deref().unwrap_or("(unnamed)");
    println!("Updated: {}", display_name);
    for change in changes {
        println!("  - {}", change);
    }

    Ok(())
}

fn find_person(db: &Database, identifier: &str) -> Result<Option<Person>> {
    // Try UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return db.get_person_by_id(uuid);
    }

    // Search by name
    let words: Vec<&str> = identifier.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, 20)?;

    match results.len() {
        0 => Ok(None),
        1 => Ok(Some(results.into_iter().next().unwrap())),
        _ => {
            // Multiple matches - show selection
            select_person(db, &results, identifier)
        }
    }
}

fn select_person(db: &Database, results: &[Person], _query: &str) -> Result<Option<Person>> {
    select_contact(db, results)
}

fn update_primary_email(db: &Database, person_id: Uuid, new_email: &str) -> Result<()> {
    let emails = db.get_emails_for_person(person_id)?;

    // Find primary or first email to update, or insert new
    let to_update = emails.iter().find(|e| e.is_primary).or(emails.first());

    if let Some(existing) = to_update {
        let mut updated = existing.clone();
        updated.email_address = new_email.to_string();
        db.update_email(&updated)?;
    } else {
        let mut email = Email::new(person_id, new_email.to_string());
        email.is_primary = true;
        db.insert_email(&email)?;
    }
    Ok(())
}

fn update_primary_phone(db: &Database, person_id: Uuid, new_phone: &str) -> Result<()> {
    let phones = db.get_phones_for_person(person_id)?;

    let to_update = phones.iter().find(|p| p.is_primary).or(phones.first());

    if let Some(existing) = to_update {
        let mut updated = existing.clone();
        updated.phone_number = new_phone.to_string();
        db.update_phone(&updated)?;
    } else {
        let mut phone = Phone::new(person_id, new_phone.to_string());
        phone.is_primary = true;
        db.insert_phone(&phone)?;
    }
    Ok(())
}

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("John".to_string());
        p.name_family = Some("Smith".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut email = Email::new(p.id, "john@example.com".to_string());
        email.is_primary = true;
        db.insert_email(&email).unwrap();

        let mut phone = Phone::new(p.id, "555-1234".to_string());
        phone.is_primary = true;
        db.insert_phone(&phone).unwrap();

        db
    }

    #[test]
    fn test_update_first_name() {
        let db = setup_test_db();

        run_update(&db, "John Smith", Some("Johnny".to_string()), None, None, None, None).unwrap();

        let results = db.search_persons_multi(&["johnny"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Johnny".to_string()));
        assert_eq!(results[0].display_name, Some("Johnny Smith".to_string()));
    }

    #[test]
    fn test_update_email() {
        let db = setup_test_db();

        run_update(&db, "John Smith", None, None, Some("newemail@example.com".to_string()), None, None).unwrap();

        let results = db.search_persons_multi(&["john"], false, 10).unwrap();
        let emails = db.get_emails_for_person(results[0].id).unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].email_address, "newemail@example.com");
    }

    #[test]
    fn test_update_requires_changes() {
        let db = setup_test_db();
        let result = run_update(&db, "John Smith", None, None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_by_uuid() {
        let db = setup_test_db();

        let results = db.search_persons_multi(&["john"], false, 1).unwrap();
        let uuid_str = results[0].id.to_string();

        run_update(&db, &uuid_str, Some("Jane".to_string()), None, None, None, None).unwrap();

        let updated = db.get_person_by_id(results[0].id).unwrap().unwrap();
        assert_eq!(updated.name_given, Some("Jane".to_string()));
    }
}
