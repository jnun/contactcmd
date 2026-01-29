use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    style::{Attribute, SetAttribute},
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};
use std::io::{self, IsTerminal, Write};

use crate::cli::display::print_full_contact;
use crate::cli::show::{run_show, show_messages_screen};
use crate::cli::ui::{clear_screen, is_valid_email, prompt_field, visible_lines, FormResult};
#[cfg(target_os = "macos")]
use crate::cli::sync::{delete_from_macos_contacts, get_apple_id};
use crate::db::Database;
use crate::models::{Email, PersonOrganization, Phone};


/// Contact data prepared for list display
pub struct ContactListRow {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub title_and_org: Option<String>,
    pub primary_email: Option<String>,
    pub primary_phone: Option<String>,
    pub location: Option<String>,
}

/// Sort field for list command
#[derive(Debug, Clone, Copy)]
pub enum SortField {
    Name,
    Created,
    Updated,
}

impl SortField {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "name" => Some(Self::Name),
            "created" => Some(Self::Created),
            "updated" => Some(Self::Updated),
            _ => None,
        }
    }

    pub fn to_sql_column(&self) -> &'static str {
        match self {
            Self::Name => "sort_name",
            Self::Created => "created_at",
            Self::Updated => "updated_at",
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "asc" => Some(Self::Asc),
            "desc" => Some(Self::Desc),
            _ => None,
        }
    }

    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// RAII guard that ensures raw mode is disabled on drop
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Execute the list command
pub fn run_list(
    db: &Database,
    page: u32,
    limit: u32,
    sort: Option<String>,
    order: String,
    all: bool,
    review: bool,
) -> Result<()> {
    let sort_field = sort
        .as_deref()
        .and_then(SortField::parse)
        .unwrap_or(SortField::Name);

    let sort_order = SortOrder::parse(&order).unwrap_or(SortOrder::Asc);

    let total = db.count_persons()?;

    if total == 0 {
        println!("No contacts.");
        return Ok(());
    }

    // Review mode - interactive one-by-one review
    if review {
        let persons = db.list_persons_sorted(
            total,
            0,
            sort_field.to_sql_column(),
            sort_order.to_sql(),
        )?;
        return run_browse_mode(db, persons);
    }

    // Use non-interactive mode if --all flag or not a TTY
    if all || !io::stdout().is_terminal() {
        let rows =
            db.list_contact_rows(total, 0, sort_field.to_sql_column(), sort_order.to_sql())?;
        println!("Contacts ({} total)\n", total);
        print_table_header();
        for row in rows {
            print_contact_row(&row, false);
        }
        return Ok(());
    }

    // Calculate dynamic limit based on terminal height (if limit is 0)
    let effective_limit = if limit == 0 {
        visible_lines() as u32
    } else {
        limit
    };

    // Interactive paginated mode (only when stdout is a TTY)
    let total_pages = total.div_ceil(effective_limit);
    let mut current_page = page.min(total_pages).max(1);

    run_interactive_list(
        db,
        &mut current_page,
        total,
        sort_field,
        sort_order,
    )
}

fn run_interactive_list(
    db: &Database,
    _current_page: &mut u32,
    total: u32,
    sort_field: SortField,
    sort_order: SortOrder,
) -> Result<()> {
    let mut cursor: usize = 0;  // Absolute index in full list
    let mut scroll: usize = 0;  // First visible row

    loop {
        clear_screen()?;

        // Recalculate visible lines on each iteration (handles terminal resize)
        let visible = visible_lines();

        // Adjust scroll to keep cursor visible
        if cursor < scroll {
            scroll = cursor;
        } else if cursor >= scroll + visible {
            scroll = cursor - visible + 1;
        }

        // Fetch only visible rows
        let rows = db.list_contact_rows(
            visible as u32,
            scroll as u32,
            sort_field.to_sql_column(),
            sort_order.to_sql(),
        )?;

        print_table_header();

        for (i, row) in rows.iter().enumerate() {
            let absolute_idx = scroll + i;
            print_contact_row(row, absolute_idx == cursor);
        }

        // Status bar
        println!("\n{}/{}  [↑↓] move [←→] page [enter] view [esc] back", cursor + 1, total);

        // Read key
        let code = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                if cursor + 1 < total as usize {
                    cursor += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                cursor = (cursor + visible).min(total as usize - 1);
            }
            KeyCode::PageUp | KeyCode::Left => {
                cursor = cursor.saturating_sub(visible);
            }
            KeyCode::Right => {
                cursor = (cursor + visible).min(total as usize - 1);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                cursor = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                cursor = total as usize - 1;
            }
            KeyCode::Enter => {
                if !rows.is_empty() {
                    let row_idx = cursor - scroll;
                    if row_idx < rows.len() {
                        let contact_id = rows[row_idx].id;
                        if run_show(db, &contact_id.to_string())? {
                            return Ok(());
                        }
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Get terminal width, defaulting to 80 if unavailable
fn get_term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Column layout based on terminal width
struct ColumnLayout {
    name_width: usize,
    contact_width: usize,
    show_location: bool,
}

impl ColumnLayout {
    fn for_width(width: usize) -> Self {
        if width >= 80 {
            // Full display: Name | Email/Phone | Location
            ColumnLayout {
                name_width: 30,
                contact_width: 28,
                show_location: true,
            }
        } else {
            // Compact display: Name | Email/Phone
            ColumnLayout {
                name_width: 25,
                contact_width: width.saturating_sub(27),
                show_location: false,
            }
        }
    }
}

fn print_table_header() {
    let layout = ColumnLayout::for_width(get_term_width());
    if layout.show_location {
        println!(
            "{:<name_w$}  {:<contact_w$}  LOCATION",
            "NAME",
            "EMAIL/PHONE",
            name_w = layout.name_width,
            contact_w = layout.contact_width
        );
    } else {
        println!(
            "{:<name_w$}  EMAIL/PHONE",
            "NAME",
            name_w = layout.name_width
        );
    }
}

fn print_contact_row(row: &ContactListRow, selected: bool) {
    let layout = ColumnLayout::for_width(get_term_width());
    let name = truncate(&row.display_name, layout.name_width);

    let contact_info = match (&row.primary_email, &row.primary_phone) {
        (Some(email), _) => truncate(email, layout.contact_width),
        (None, Some(phone)) => truncate(phone, layout.contact_width),
        (None, None) => String::new(),
    };

    let line = if layout.show_location {
        let location = row
            .location
            .as_ref()
            .map(|l| truncate(l, 20))
            .unwrap_or_default();
        format!(
            "{:<name_w$}  {:<contact_w$}  {}",
            name,
            contact_info,
            location,
            name_w = layout.name_width,
            contact_w = layout.contact_width
        )
    } else {
        format!(
            "{:<name_w$}  {}",
            name,
            contact_info,
            name_w = layout.name_width
        )
    };

    if selected {
        let mut stdout = io::stdout();
        let _ = stdout.execute(SetAttribute(Attribute::Reverse));
        print!("{}", line);
        let _ = stdout.execute(SetAttribute(Attribute::Reset));
        println!();
    } else {
        println!("{}", line);
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

// ============================================================================
// Review/Browse Mode Implementation
// ============================================================================

/// Run the interactive browse/review mode with a pre-fetched list of persons
pub fn run_browse_mode(db: &Database, persons: Vec<crate::models::Person>) -> Result<()> {
    if persons.is_empty() {
        println!("No contacts to browse.");
        return Ok(());
    }

    let mut index = 0;

    while index < persons.len() {
        let person = &persons[index];

        // Get full contact detail for display
        let detail = match db.get_contact_detail(person.id)? {
            Some(d) => d,
            None => {
                index += 1;
                continue;
            }
        };

        clear_screen()?;
        print_full_contact(&detail, None);

        print!("\n{}/{}  [e]dit [m]essages [d]elete [←/→] [q]uit: ", index + 1, persons.len());
        io::stdout().flush()?;

        // Use raw mode for immediate single-key response
        let action = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match action {
            KeyCode::Char('e') | KeyCode::Char('E') => {
                println!();
                if handle_edit(db, &detail)? {
                    index += 1;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                println!();
                if handle_edit_all(db, &detail)? {
                    index += 1;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                println!();
                handle_notes(db, &detail)?;
                index += 1;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                println!();
                if show_messages_screen(db, &detail)? {
                    break; // Quit requested from messages screen
                }
                // Continue showing same contact after returning from messages
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Delete immediately without confirmation
                let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                println!();

                // Delete from macOS Contacts if synced from there
                #[cfg(target_os = "macos")]
                if let Some(apple_id) = get_apple_id(&detail.person) {
                    if let Err(e) = delete_from_macos_contacts(&apple_id) {
                        eprintln!("Warning: Could not delete from macOS Contacts: {}", e);
                    } else {
                        println!("Deleted from macOS Contacts");
                    }
                }

                if db.delete_person(detail.person.id)? {
                    println!("Deleted: {}", display_name);
                }
                index += 1;
            }
            KeyCode::Right | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter | KeyCode::Char(' ') => {
                index += 1;
            }
            KeyCode::Left => {
                index = index.saturating_sub(1);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                break;
            }
            _ => {
                // Unknown input, stay on same contact
            }
        }
    }

    clear_screen()?;

    if index >= persons.len() {
        println!("Reviewed {} contacts.", persons.len());
    }

    Ok(())
}

/// Handle edit action - prompt for each field
pub fn handle_edit(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<bool> {
    let person = &detail.person;

    // First name
    let current_first = person.name_given.as_deref().unwrap_or("");
    let new_first = match prompt_field("first", Some(current_first))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(false);
        }
    };

    // Last name
    let current_last = person.name_family.as_deref().unwrap_or("");
    let new_last = match prompt_field("last", Some(current_last))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(false);
        }
    };

    // Email (primary)
    let current_email = detail
        .emails
        .iter()
        .find(|e| e.is_primary)
        .or(detail.emails.first())
        .map(|e| e.email_address.as_str())
        .unwrap_or("");
    let new_email = match prompt_field("email", Some(current_email))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(false);
        }
    };

    // Validate email if changed and not empty
    if !new_email.is_empty() && new_email != current_email && !is_valid_email(&new_email) {
        eprintln!("Invalid email format");
        return Ok(false);
    }

    // Phone (primary)
    let current_phone = detail
        .phones
        .iter()
        .find(|p| p.is_primary)
        .or(detail.phones.first())
        .map(|p| p.phone_number.as_str())
        .unwrap_or("");
    let new_phone = match prompt_field("phone", Some(current_phone))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(false);
        }
    };

    // Notes
    let current_notes = person.notes.as_deref().unwrap_or("");
    let new_notes = match prompt_field("notes", Some(current_notes))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(false);
        }
    };

    // Apply changes
    let mut updated_person = person.clone();
    let mut has_changes = false;

    if new_first != current_first {
        updated_person.name_given = if new_first.is_empty() { None } else { Some(new_first) };
        has_changes = true;
    }
    if new_last != current_last {
        updated_person.name_family = if new_last.is_empty() { None } else { Some(new_last) };
        has_changes = true;
    }
    if new_notes != current_notes {
        updated_person.notes = if new_notes.is_empty() { None } else { Some(new_notes) };
        has_changes = true;
    }

    // Recompute display names if name changed
    if updated_person.name_given != person.name_given
        || updated_person.name_family != person.name_family
    {
        updated_person.compute_names();
    }

    // Update person record if changed
    if updated_person.name_given != person.name_given
        || updated_person.name_family != person.name_family
        || updated_person.notes != person.notes
    {
        db.update_person(&updated_person)?;
    }

    // Handle email update
    if new_email != current_email {
        update_primary_email(db, person.id, &new_email, &detail.emails)?;
        has_changes = true;
    }

    // Handle phone update
    if new_phone != current_phone {
        update_primary_phone(db, person.id, &new_phone, &detail.phones)?;
        has_changes = true;
    }

    if has_changes {
        println!("\nSaved.");
    }

    Ok(true)
}

/// Handle edit all fields action - comprehensive field editing
pub fn handle_edit_all(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<bool> {
    let person = &detail.person;

    // Helper macro to reduce repetition
    macro_rules! get_field {
        ($field:expr, $current:expr) => {
            match prompt_field($field, Some($current))? {
                FormResult::Value(v) => v,
                FormResult::Cancelled => {
                    println!("Cancelled.");
                    return Ok(false);
                }
            }
        };
    }

    // Name fields
    let current_first = person.name_given.as_deref().unwrap_or("");
    let new_first = get_field!("first", current_first);

    let current_last = person.name_family.as_deref().unwrap_or("");
    let new_last = get_field!("last", current_last);

    let current_middle = person.name_middle.as_deref().unwrap_or("");
    let new_middle = get_field!("middle", current_middle);

    let current_nickname = person.name_nickname.as_deref().unwrap_or("");
    let new_nickname = get_field!("nickname", current_nickname);

    let current_prefix = person.name_prefix.as_deref().unwrap_or("");
    let new_prefix = get_field!("prefix", current_prefix);

    let current_suffix = person.name_suffix.as_deref().unwrap_or("");
    let new_suffix = get_field!("suffix", current_suffix);

    // Organization fields
    let (current_company, current_title, current_dept) = detail
        .organizations
        .first()
        .map(|(po, org)| {
            (
                org.name.as_str(),
                po.title.as_deref().unwrap_or(""),
                po.department.as_deref().unwrap_or(""),
            )
        })
        .unwrap_or(("", "", ""));

    let new_company = get_field!("company", current_company);
    let new_title = get_field!("title", current_title);
    let new_dept = get_field!("department", current_dept);

    // Emails - edit each one
    let mut new_emails: Vec<(uuid::Uuid, String, String)> = Vec::new();
    for email in &detail.emails {
        let type_label = email.email_type.as_str();
        let field_name = format!("email {}", type_label);
        let new_val = get_field!(&field_name, &email.email_address);

        if !new_val.is_empty() && new_val != email.email_address && !is_valid_email(&new_val) {
            eprintln!("Invalid email format");
            new_emails.push((email.id, email.email_address.clone(), type_label.to_string()));
        } else {
            new_emails.push((email.id, new_val, type_label.to_string()));
        }
    }

    // Phones - edit each one
    let mut new_phones: Vec<(uuid::Uuid, String, String)> = Vec::new();
    for phone in &detail.phones {
        let type_label = phone.phone_type.as_str();
        let field_name = format!("phone {}", type_label);
        let new_val = get_field!(&field_name, &phone.phone_number);
        new_phones.push((phone.id, new_val, type_label.to_string()));
    }

    // Notes
    let current_notes = person.notes.as_deref().unwrap_or("");
    let new_notes = get_field!("notes", current_notes);

    // Apply person changes
    let mut updated_person = person.clone();
    let mut has_changes = false;

    if new_first != current_first {
        updated_person.name_given = non_empty(new_first);
        has_changes = true;
    }
    if new_last != current_last {
        updated_person.name_family = non_empty(new_last);
        has_changes = true;
    }
    if new_middle != current_middle {
        updated_person.name_middle = non_empty(new_middle);
        has_changes = true;
    }
    if new_nickname != current_nickname {
        updated_person.name_nickname = non_empty(new_nickname);
        has_changes = true;
    }
    if new_prefix != current_prefix {
        updated_person.name_prefix = non_empty(new_prefix);
        has_changes = true;
    }
    if new_suffix != current_suffix {
        updated_person.name_suffix = non_empty(new_suffix);
        has_changes = true;
    }
    if new_notes != current_notes {
        updated_person.notes = non_empty(new_notes);
        has_changes = true;
    }

    // Recompute display names if any name field changed
    let name_changed = updated_person.name_prefix != person.name_prefix
        || updated_person.name_given != person.name_given
        || updated_person.name_middle != person.name_middle
        || updated_person.name_family != person.name_family
        || updated_person.name_suffix != person.name_suffix
        || updated_person.name_nickname != person.name_nickname;

    if name_changed {
        updated_person.compute_names();
    }

    // Update person record if changed
    if name_changed || updated_person.notes != person.notes {
        db.update_person(&updated_person)?;
    }

    // Handle organization changes
    let org_changed = new_company != current_company
        || new_title != current_title
        || new_dept != current_dept;

    if org_changed {
        db.delete_person_organizations(person.id)?;

        if !new_company.is_empty() {
            let org = db.get_or_create_organization(&new_company)?;
            let mut po = PersonOrganization::new(person.id, org.id);
            po.title = non_empty(new_title);
            po.department = non_empty(new_dept);
            po.is_primary = true;
            db.insert_person_organization(&po)?;
        }
        has_changes = true;
    }

    // Handle email updates
    for (i, (_id, new_val, _type_str)) in new_emails.iter().enumerate() {
        let original = &detail.emails[i];
        if *new_val != original.email_address {
            let mut updated_email = original.clone();
            updated_email.email_address = new_val.clone();
            db.update_email(&updated_email)?;
            has_changes = true;
        }
    }

    // Handle phone updates
    for (i, (_id, new_val, _type_str)) in new_phones.iter().enumerate() {
        let original = &detail.phones[i];
        if *new_val != original.phone_number {
            let mut updated_phone = original.clone();
            updated_phone.phone_number = new_val.clone();
            db.update_phone(&updated_phone)?;
            has_changes = true;
        }
    }

    if has_changes {
        println!("\nSaved.");
    }

    Ok(true)
}

/// Convert empty string to None
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Handle notes editing
pub fn handle_notes(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<()> {
    let person = &detail.person;
    let current_notes = person.notes.as_deref().unwrap_or("");

    let new_notes = match prompt_field("notes", Some(current_notes))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(());
        }
    };

    if new_notes != current_notes {
        let mut updated = person.clone();
        updated.notes = if new_notes.is_empty() { None } else { Some(new_notes) };
        db.update_person(&updated)?;
        println!("\nSaved.");
    }

    Ok(())
}

/// Update primary email for a person
fn update_primary_email(
    db: &Database,
    person_id: uuid::Uuid,
    new_email: &str,
    existing_emails: &[Email],
) -> Result<()> {
    // Find primary or first email to update, or insert new
    let to_update = existing_emails
        .iter()
        .find(|e| e.is_primary)
        .or(existing_emails.first());

    if new_email.is_empty() {
        // Don't delete existing email, just leave it
        return Ok(());
    }

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

/// Update primary phone for a person
fn update_primary_phone(
    db: &Database,
    person_id: uuid::Uuid,
    new_phone: &str,
    existing_phones: &[Phone],
) -> Result<()> {
    let to_update = existing_phones
        .iter()
        .find(|p| p.is_primary)
        .or(existing_phones.first());

    if new_phone.is_empty() {
        // Don't delete existing phone, just leave it
        return Ok(());
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Email, Person, Phone};

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        let names = vec![
            ("John", "Smith"),
            ("Jane", "Doe"),
            ("Alice", "Johnson"),
            ("Bob", "Williams"),
            ("Charlie", "Brown"),
        ];

        for (first, last) in names {
            let mut p = Person::new();
            p.name_given = Some(first.to_string());
            p.name_family = Some(last.to_string());
            p.compute_names();
            db.insert_person(&p).unwrap();
        }

        db
    }

    #[test]
    fn test_list_contact_rows() {
        let db = setup_test_db();
        let rows = db.list_contact_rows(10, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn test_list_contact_rows_pagination() {
        let db = setup_test_db();
        let rows = db.list_contact_rows(2, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 2);

        let rows2 = db.list_contact_rows(2, 2, "sort_name", "ASC").unwrap();
        assert_eq!(rows2.len(), 2);
    }

    #[test]
    fn test_list_contact_rows_with_details() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Test".to_string());
        p.name_family = Some("User".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut email = Email::new(p.id, "test@example.com".to_string());
        email.is_primary = true;
        db.insert_email(&email).unwrap();

        let mut phone = Phone::new(p.id, "555-1234".to_string());
        phone.is_primary = true;
        db.insert_phone(&phone).unwrap();

        let mut addr = Address::new(p.id);
        addr.city = Some("Austin".to_string());
        addr.state = Some("TX".to_string());
        addr.is_primary = true;
        db.insert_address(&addr).unwrap();

        let rows = db.list_contact_rows(10, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].display_name, "Test User");
        assert_eq!(rows[0].primary_email, Some("test@example.com".to_string()));
        assert_eq!(rows[0].primary_phone, Some("555-1234".to_string()));
        assert_eq!(rows[0].location, Some("Austin, TX".to_string()));
    }

    #[test]
    fn test_sort_field_from_str() {
        assert!(matches!(SortField::parse("name"), Some(SortField::Name)));
        assert!(matches!(
            SortField::parse("created"),
            Some(SortField::Created)
        ));
        assert!(matches!(
            SortField::parse("updated"),
            Some(SortField::Updated)
        ));
        assert!(SortField::parse("invalid").is_none());
    }

    #[test]
    fn test_sort_order_from_str() {
        assert!(matches!(SortOrder::parse("asc"), Some(SortOrder::Asc)));
        assert!(matches!(SortOrder::parse("desc"), Some(SortOrder::Desc)));
        assert!(SortOrder::parse("invalid").is_none());
    }
}
