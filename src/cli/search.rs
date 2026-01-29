use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

use crate::db::Database;
use crate::models::Person;
use super::display::print_full_contact;
use super::list::{handle_edit, handle_edit_all, handle_notes};
use super::messages::get_last_message_for_handles;
use super::show::show_messages_screen;
use super::ui::clear_screen;
#[cfg(target_os = "macos")]
use super::sync::{delete_from_macos_contacts, get_apple_id};

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

/// Execute the search command
pub fn run_search(db: &Database, query: &str, case_sensitive: bool, missing: Option<&str>) -> Result<()> {
    // Handle --missing flag
    if let Some(field) = missing {
        return run_missing_search(db, field);
    }

    let query = query.trim();
    if query.is_empty() {
        println!("No query.");
        return Ok(());
    }

    let words: Vec<&str> = query.split_whitespace().collect();
    let results = db.search_persons_multi(&words, case_sensitive, u32::MAX)?;

    if results.is_empty() {
        println!("No matches.");
        return Ok(());
    }

    // Always use review mode to display results
    run_search_review_mode(db, &results, query)
}

/// Search for contacts missing phone or email
fn run_missing_search(db: &Database, field: &str) -> Result<()> {
    let (results, label) = match field.to_lowercase().as_str() {
        "phone" => (db.find_persons_missing_phone(u32::MAX)?, "a phone number"),
        "email" => (db.find_persons_missing_email(u32::MAX)?, "an email address"),
        "contact" => (db.find_persons_missing_both(u32::MAX)?, "contact info"),
        _ => {
            anyhow::bail!("Invalid --missing value: \"{}\". Use \"phone\", \"email\", or \"contact\".", field);
        }
    };

    if results.is_empty() {
        println!("All contacts have {}.", label);
        return Ok(());
    }

    let query = format!("missing {}", field);
    run_search_review_mode(db, &results, &query)
}

#[allow(dead_code)]
fn show_person_detail(db: &Database, person: &Person) -> Result<()> {
    match db.get_contact_detail(person.id)? {
        Some(detail) => {
            // Extract phone numbers and emails for message lookup
            let phones: Vec<String> = detail.phones.iter()
                .map(|p| p.phone_number.clone())
                .collect();
            let emails: Vec<String> = detail.emails.iter()
                .map(|e| e.email_address.clone())
                .collect();

            // Try to get the last message (gracefully handle errors)
            let last_message = get_last_message_for_handles(&phones, &emails).ok().flatten();

            print_full_contact(&detail, last_message.as_ref());
        }
        None => {
            eprintln!("Warning: Could not load details for {}", person.id);
        }
    }
    Ok(())
}

/// Run interactive review mode for search results
fn run_search_review_mode(db: &Database, results: &[Person], _query: &str) -> Result<()> {
    let mut index = 0;

    while index < results.len() {
        let person = &results[index];

        // Get full contact detail for display
        let detail = match db.get_contact_detail(person.id)? {
            Some(d) => d,
            None => {
                index += 1;
                continue;
            }
        };

        clear_screen()?;

        // Extract phone numbers and emails for message lookup
        let phones: Vec<String> = detail.phones.iter()
            .map(|p| p.phone_number.clone())
            .collect();
        let emails: Vec<String> = detail.emails.iter()
            .map(|e| e.email_address.clone())
            .collect();
        let last_message = get_last_message_for_handles(&phones, &emails).ok().flatten();

        print_full_contact(&detail, last_message.as_ref());

        print!("\n{}/{}  [e]dit [m]essages [d]elete [←/→] [q]uit: ", index + 1, results.len());
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
            KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('s') | KeyCode::Char('S') => {
                index += 1;
            }
            KeyCode::Left | KeyCode::Char('p') | KeyCode::Char('P') => {
                if index > 0 {
                    index -= 1;
                }
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

    if index >= results.len() {
        println!("Reviewed {} contacts.", results.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Email, Note, Organization, PersonOrganization};

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        // Add test contacts
        let contacts = vec![
            ("John", "Smith", Some("john@example.com")),
            ("Jane", "Smith", Some("jane@gmail.com")),
            ("John", "Doe", Some("jdoe@work.com")),
            ("Alice", "Johnson", None),
        ];

        for (first, last, email) in contacts {
            let mut p = Person::new();
            p.name_given = Some(first.to_string());
            p.name_family = Some(last.to_string());
            p.compute_names();
            db.insert_person(&p).unwrap();

            if let Some(addr) = email {
                let mut e = Email::new(p.id, addr.to_string());
                e.is_primary = true;
                db.insert_email(&e).unwrap();
            }
        }

        db
    }

    #[test]
    fn test_search_single_word() {
        let db = setup_test_db();
        let results = db.search_persons_multi(&["john"], false, 10).unwrap();
        // John Smith, John Doe, Alice Johnson (johnson contains john)
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_multi_word_and() {
        let db = setup_test_db();
        let results = db
            .search_persons_multi(&["john", "smith"], false, 10)
            .unwrap();
        assert_eq!(results.len(), 1); // Only John Smith matches both
        assert_eq!(results[0].name_given, Some("John".to_string()));
        assert_eq!(results[0].name_family, Some("Smith".to_string()));
    }

    #[test]
    fn test_search_email() {
        let db = setup_test_db();
        let results = db.search_persons_multi(&["@gmail"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Jane".to_string()));
    }

    #[test]
    fn test_search_case_insensitive() {
        let db = setup_test_db();
        let results = db.search_persons_multi(&["JOHN"], false, 10).unwrap();
        // Case-insensitive: finds john in John Smith, John Doe, Alice Johnson
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_case_sensitive() {
        let db = setup_test_db();
        // GLOB is case-sensitive: "JOHN" won't match "John" or "johnson"
        let results = db.search_persons_multi(&["JOHN"], true, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_results() {
        let db = setup_test_db();
        let results = db
            .search_persons_multi(&["nonexistent"], false, 10)
            .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_limit() {
        let db = setup_test_db();
        let results = db.search_persons_multi(&["smith"], false, 1).unwrap();
        assert_eq!(results.len(), 1); // Limited to 1 even though 2 match
    }

    #[test]
    fn test_search_glob_metacharacters() {
        let db = Database::open_memory().unwrap();

        // Create contact with literal asterisk in name
        let mut p = Person::new();
        p.name_given = Some("Test*User".to_string());
        p.name_family = Some("Star".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        // Search for literal asterisk (case-sensitive uses GLOB)
        let results = db.search_persons_multi(&["*"], true, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Test*User".to_string()));

        // Without escaping, "*" would match everything - verify it doesn't
        let results2 = db.search_persons_multi(&["Test*User"], true, 10).unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_search_like_metacharacters() {
        let db = Database::open_memory().unwrap();

        // Create contact with percent sign in email
        let mut p = Person::new();
        p.name_given = Some("Percent".to_string());
        p.name_family = Some("Test".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut e = Email::new(p.id, "100%off@deals.com".to_string());
        e.is_primary = true;
        db.insert_email(&e).unwrap();

        // Search for literal percent (case-insensitive uses LIKE)
        let results = db.search_persons_multi(&["%off"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_batch_display_info() {
        let db = Database::open_memory().unwrap();

        // Create two contacts with emails and addresses
        let mut p1 = Person::new();
        p1.name_given = Some("Alice".to_string());
        p1.compute_names();
        db.insert_person(&p1).unwrap();

        let mut e1 = Email::new(p1.id, "alice@test.com".to_string());
        e1.is_primary = true;
        db.insert_email(&e1).unwrap();

        let mut p2 = Person::new();
        p2.name_given = Some("Bob".to_string());
        p2.compute_names();
        db.insert_person(&p2).unwrap();

        let mut a2 = Address::new(p2.id);
        a2.city = Some("Austin".to_string());
        a2.state = Some("TX".to_string());
        a2.is_primary = true;
        db.insert_address(&a2).unwrap();

        // Batch fetch
        let info = db.get_display_info_for_persons(&[p1.id, p2.id]).unwrap();

        assert_eq!(info.len(), 2);
        assert_eq!(
            info.get(&p1.id).unwrap().0,
            Some("alice@test.com".to_string())
        );
        assert_eq!(info.get(&p1.id).unwrap().1, None);
        assert_eq!(info.get(&p2.id).unwrap().0, None);
        assert_eq!(info.get(&p2.id).unwrap().1, Some("Austin, TX".to_string()));
    }

    #[test]
    fn test_search_by_notes() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Sarah".to_string());
        p.name_family = Some("Connor".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let note = Note::new(p.id, "Met at conference in Berlin".to_string());
        db.insert_note(&note).unwrap();

        // Search by note content
        let results = db.search_persons_multi(&["conference"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Sarah".to_string()));

        // Search by note content (different word)
        let results = db.search_persons_multi(&["berlin"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // No match
        let results = db.search_persons_multi(&["paris"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_city() {
        let db = Database::open_memory().unwrap();

        let mut p1 = Person::new();
        p1.name_given = Some("Austin".to_string()); // Name is Austin
        p1.name_family = Some("Powers".to_string());
        p1.compute_names();
        db.insert_person(&p1).unwrap();

        let mut p2 = Person::new();
        p2.name_given = Some("Bob".to_string());
        p2.name_family = Some("Builder".to_string());
        p2.compute_names();
        db.insert_person(&p2).unwrap();

        let mut addr = Address::new(p2.id);
        addr.city = Some("Austin".to_string()); // Lives in Austin
        db.insert_address(&addr).unwrap();

        // Search for "austin" should find both
        let results = db.search_persons_multi(&["austin"], false, 10).unwrap();
        assert_eq!(results.len(), 2);

        // Search for "austin builder" should only find Bob (name + city)
        let results = db.search_persons_multi(&["austin", "builder"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Bob".to_string()));
    }

    #[test]
    fn test_search_by_organization() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Tony".to_string());
        p.name_family = Some("Stark".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let org = Organization::new("Stark Industries".to_string());
        db.insert_organization(&org).unwrap();

        let po = PersonOrganization::new(p.id, org.id);
        db.insert_person_organization(&po).unwrap();

        // Search by company name
        let results = db.search_persons_multi(&["stark"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        let results = db.search_persons_multi(&["industries"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // No match
        let results = db.search_persons_multi(&["wayne"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_combined_fields() {
        let db = Database::open_memory().unwrap();

        // Create person with all searchable fields
        let mut p = Person::new();
        p.name_given = Some("Bruce".to_string());
        p.name_family = Some("Wayne".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut addr = Address::new(p.id);
        addr.city = Some("Gotham".to_string());
        db.insert_address(&addr).unwrap();

        let org = Organization::new("Wayne Enterprises".to_string());
        db.insert_organization(&org).unwrap();
        let po = PersonOrganization::new(p.id, org.id);
        db.insert_person_organization(&po).unwrap();

        let note = Note::new(p.id, "Definitely not Batman".to_string());
        db.insert_note(&note).unwrap();

        // Multi-word search across different fields: name + city + note
        let results = db.search_persons_multi(&["bruce", "gotham", "batman"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // Search: name + organization
        let results = db.search_persons_multi(&["wayne", "enterprises"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // Should not match if one word doesn't exist
        let results = db.search_persons_multi(&["bruce", "metropolis"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
