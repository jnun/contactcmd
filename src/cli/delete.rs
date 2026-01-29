use anyhow::{anyhow, Result};
use inquire::Confirm;
use uuid::Uuid;

#[cfg(target_os = "macos")]
use crate::cli::sync::{delete_from_macos_contacts, get_apple_id};
use crate::cli::ui::{minimal_render_config, select_contact};
use crate::db::Database;
use crate::models::Person;

/// Execute the delete command
pub fn run_delete(db: &Database, identifier: &str, force: bool) -> Result<()> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Identifier cannot be empty."));
    }

    // Try parsing as UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return delete_by_uuid(db, uuid, identifier, force);
    }

    // Not a UUID, search by name
    let words: Vec<&str> = identifier.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, 20)?;

    match results.len() {
        0 => {
            println!("No matches.");
        }
        1 => {
            delete_person_with_confirm(db, &results[0], force)?;
        }
        _ => {
            run_selection_menu(db, &results, identifier, force)?;
        }
    }

    Ok(())
}

fn delete_by_uuid(db: &Database, uuid: Uuid, identifier: &str, force: bool) -> Result<()> {
    match db.get_person_by_id(uuid)? {
        Some(person) => {
            delete_person_with_confirm(db, &person, force)?;
        }
        None => {
            println!("No contact found with ID: {}", identifier);
        }
    }
    Ok(())
}

fn delete_person_with_confirm(db: &Database, person: &Person, force: bool) -> Result<()> {
    let display_name = person.display_name.as_deref().unwrap_or("(unnamed)");

    print_contact_summary(db, person)?;
    println!();

    if !force {
        let confirmed = Confirm::new(&format!("Delete {}?", display_name))
            .with_render_config(minimal_render_config())
            .with_default(false)
            .prompt()
            .unwrap_or(false);

        if !confirmed {
            return Ok(());
        }
    }

    // Delete from macOS Contacts if synced from there
    #[cfg(target_os = "macos")]
    if let Some(apple_id) = get_apple_id(person) {
        if let Err(e) = delete_from_macos_contacts(&apple_id) {
            eprintln!("Warning: Could not delete from macOS Contacts: {}", e);
        }
    }

    // Perform the delete
    if db.delete_person(person.id)? {
        println!("Deleted.");
    } else {
        eprintln!("Error: failed to delete {}", display_name);
    }

    Ok(())
}

fn print_contact_summary(db: &Database, person: &Person) -> Result<()> {
    let display_name = person.display_name.as_deref().unwrap_or("(unnamed)");
    println!("{}", display_name);

    // Primary email
    let emails = db.get_emails_for_person(person.id)?;
    if let Some(email) = emails.iter().find(|e| e.is_primary).or(emails.first()) {
        println!("  {}", email.email_address);
    }

    // Current org/title
    let orgs = db.get_organizations_for_person(person.id)?;
    for (po, org) in &orgs {
        if po.is_current {
            match &po.title {
                Some(title) => println!("  {} at {}", title, org.name),
                None => println!("  {}", org.name),
            }
            break;
        }
    }

    // Location
    let addresses = db.get_addresses_for_person(person.id)?;
    if let Some(addr) = addresses.iter().find(|a| a.is_primary).or(addresses.first()) {
        if let Some(loc) = addr.city_state() {
            println!("  {}", loc);
        }
    }

    Ok(())
}

fn run_selection_menu(db: &Database, results: &[Person], _query: &str, force: bool) -> Result<()> {
    if let Some(person) = select_contact(db, results)? {
        delete_person_with_confirm(db, &person, force)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Email;

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

        db
    }

    #[test]
    fn test_delete_by_uuid_force() {
        let db = setup_test_db();

        let results = db.search_persons_multi(&["john"], false, 1).unwrap();
        let uuid = results[0].id;

        // Force delete (no confirmation)
        assert!(db.delete_person(uuid).unwrap());

        // Verify deleted
        let person = db.get_person_by_id(uuid).unwrap();
        assert!(person.is_none());
    }

    #[test]
    fn test_cascade_delete() {
        let db = setup_test_db();

        let results = db.search_persons_multi(&["john"], false, 1).unwrap();
        let uuid = results[0].id;

        // Verify email exists
        let emails = db.get_emails_for_person(uuid).unwrap();
        assert_eq!(emails.len(), 1);

        // Delete person
        db.delete_person(uuid).unwrap();

        // Emails should be cascade deleted
        let emails = db.get_emails_for_person(uuid).unwrap();
        assert_eq!(emails.len(), 0);
    }

    #[test]
    fn test_empty_identifier_error() {
        let db = setup_test_db();
        let result = run_delete(&db, "   ", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent() {
        let db = setup_test_db();
        let fake_uuid = Uuid::new_v4();
        let deleted = db.delete_person(fake_uuid).unwrap();
        assert!(!deleted);
    }
}
