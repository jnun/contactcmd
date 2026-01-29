use anyhow::{anyhow, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};
use uuid::Uuid;

use crate::db::Database;
use crate::models::{ContactDetail, Person};
use super::display::print_full_contact;
use super::list::{handle_edit, handle_edit_all, handle_notes};
use super::display::format_message_date;
use super::messages::{get_last_message_for_handles, get_messages_for_handles};
use super::ui::{clear_screen, select_contact, visible_lines};
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

/// Execute the show command
/// Returns Ok(true) if user wants to quit the app
pub fn run_show(db: &Database, identifier: &str) -> Result<bool> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Identifier cannot be empty."));
    }

    // Try parsing as UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return show_by_uuid(db, uuid, identifier);
    }

    // Not a UUID, search by name (no limit for interactive mode)
    let words: Vec<&str> = identifier.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, u32::MAX)?;

    match results.len() {
        0 => {
            println!("No matches.");
            Ok(false)
        }
        1 => {
            show_person_detail(db, &results[0])
        }
        _ => {
            run_selection_menu(db, &results, identifier)
        }
    }
}

fn show_by_uuid(db: &Database, uuid: Uuid, identifier: &str) -> Result<bool> {
    match db.get_contact_detail(uuid)? {
        Some(detail) => {
            interactive_display(db, &detail)
        }
        None => {
            println!("No contact found with ID: {}", identifier);
            Ok(false)
        }
    }
}

/// Print a contact with their last message (if available)
fn print_contact_with_message(detail: &ContactDetail) {
    // Extract phone numbers and emails for message lookup
    let phones: Vec<String> = detail.phones.iter()
        .map(|p| p.phone_number.clone())
        .collect();
    let emails: Vec<String> = detail.emails.iter()
        .map(|e| e.email_address.clone())
        .collect();

    // Try to get the last message (gracefully handle errors)
    let last_message = get_last_message_for_handles(&phones, &emails).ok().flatten();

    print_full_contact(detail, last_message.as_ref());
}

/// Interactive display with edit/delete/notes actions
/// Returns Ok(true) if user wants to quit the app
fn interactive_display(db: &Database, detail: &ContactDetail) -> Result<bool> {
    let mut quit_app = false;

    loop {
        clear_screen()?;
        print_contact_with_message(detail);

        print!("\n[e]dit [m]essages [d]elete [q]uit: ");
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
                handle_edit(db, detail)?;
                // Refresh display after edit
                if let Some(updated) = db.get_contact_detail(detail.person.id)? {
                    return interactive_display(db, &updated);
                }
                break;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                println!();
                handle_edit_all(db, detail)?;
                // Refresh display after edit
                if let Some(updated) = db.get_contact_detail(detail.person.id)? {
                    return interactive_display(db, &updated);
                }
                break;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                println!();
                handle_notes(db, detail)?;
                // Refresh display after edit
                if let Some(updated) = db.get_contact_detail(detail.person.id)? {
                    return interactive_display(db, &updated);
                }
                break;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                println!();
                if show_messages_screen(db, detail)? {
                    clear_screen()?;
                    quit_app = true;
                    break; // Quit requested from messages screen
                }
                return interactive_display(db, detail);
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                println!();

                // Delete from macOS Contacts if synced from there
                #[cfg(target_os = "macos")]
                if let Some(apple_id) = get_apple_id(&detail.person) {
                    if let Err(e) = delete_from_macos_contacts(&apple_id) {
                        eprintln!("Warning: Could not delete from macOS Contacts: {}", e);
                    }
                }

                if db.delete_person(detail.person.id)? {
                    clear_screen()?;
                    println!("Deleted: {}", display_name);
                }
                break;
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                clear_screen()?;
                quit_app = true;
                break;
            }
            KeyCode::Enter => {
                clear_screen()?;
                break;
            }
            _ => {
                // Unknown input, redisplay
            }
        }
    }

    Ok(quit_app)
}

/// Show a scrollable messages screen for a contact with selection support
/// Returns true if user wants to quit the app entirely
pub fn show_messages_screen(_db: &Database, detail: &ContactDetail) -> Result<bool> {
    // Collect both phone numbers and email addresses as potential message handles
    let phones: Vec<String> = detail.phones.iter()
        .map(|p| p.phone_number.clone())
        .collect();
    let emails: Vec<String> = detail.emails.iter()
        .map(|e| e.email_address.clone())
        .collect();

    let messages = get_messages_for_handles(&phones, &emails, 50)?;

    if messages.is_empty() {
        clear_screen()?;
        println!("No messages for this contact.\n");
        print!("[enter] to go back: ");
        io::stdout().flush()?;

        // Wait for key press before returning
        let _action = {
            let _guard = RawModeGuard::new()?;
            event::read()?
        };
        return Ok(false);
    }

    let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
    let total_msgs = messages.len();
    let mut selected: usize = 0;

    loop {
        clear_screen()?;
        let num_visible = visible_lines(); // Recalculate for resize support

        // Calculate scroll to keep selection visible
        let scroll = if selected < num_visible / 2 {
            0
        } else if selected + num_visible / 2 >= total_msgs {
            total_msgs.saturating_sub(num_visible)
        } else {
            selected.saturating_sub(num_visible / 2)
        };

        println!("Messages: {}\n", display_name);

        let end = std::cmp::min(scroll + num_visible, total_msgs);
        for (i, msg) in messages[scroll..end].iter().enumerate() {
            let idx = scroll + i;
            let marker = if idx == selected { ">" } else { " " };
            let direction = if msg.is_from_me { "→" } else { "←" };
            let date_str = format_message_date(&msg.date);
            let first_line = msg.text.lines().next().unwrap_or("").trim();
            let text = if first_line.chars().count() <= 50 {
                first_line.to_string()
            } else {
                format!("{}…", first_line.chars().take(49).collect::<String>())
            };
            println!("{} {} {} \"{}\"", marker, direction, date_str, text);
        }

        print!("\n{}/{}  [↑/↓] select [enter] view [q]uit", selected + 1, total_msgs);
        io::stdout().flush()?;

        // Read key
        let action = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match action {
            KeyCode::Down | KeyCode::Char('j') => {
                if selected + 1 < total_msgs {
                    selected += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            KeyCode::Enter => {
                if show_full_message(&messages[selected], display_name)? {
                    return Ok(true); // Propagate quit
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                return Ok(true); // Quit
            }
            _ => {}
        }
    }
}

/// Show a single message in full
/// Returns true if user pressed quit (q), false if they pressed back (enter)
fn show_full_message(msg: &super::messages::LastMessage, contact_name: &str) -> Result<bool> {
    clear_screen()?;

    let direction = if msg.is_from_me { "To" } else { "From" };
    let date_str = format_message_date(&msg.date);

    println!("{}: {}", direction, contact_name);
    println!("{}\n", date_str);
    println!("{}", msg.text);

    print!("\n[q]uit [enter] back");
    io::stdout().flush()?;

    // Wait for key press
    let quit = {
        let _guard = RawModeGuard::new()?;
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                break matches!(code, KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc);
            }
        }
    };

    Ok(quit)
}

fn show_person_detail(db: &Database, person: &Person) -> Result<bool> {
    match db.get_contact_detail(person.id)? {
        Some(detail) => interactive_display(db, &detail),
        None => {
            // This shouldn't happen - we just found this person via search
            eprintln!("Warning: Could not load details for {}", person.id);
            Ok(false)
        }
    }
}

fn run_selection_menu(db: &Database, results: &[Person], _query: &str) -> Result<bool> {
    if let Some(person) = select_contact(db, results)? {
        return show_person_detail(db, &person);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Email, Phone, Address, AddressType, EmailType, PhoneType};

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        // Create contacts
        let contacts = vec![
            ("John", "Smith"),
            ("Jane", "Smith"),
            ("John", "Doe"),
        ];

        for (first, last) in contacts {
            let mut p = Person::new();
            p.name_given = Some(first.to_string());
            p.name_family = Some(last.to_string());
            p.compute_names();
            db.insert_person(&p).unwrap();

            // Add email for first contact
            if first == "John" && last == "Smith" {
                let mut email = Email::new(p.id, "john@example.com".to_string());
                email.email_type = EmailType::Work;
                email.is_primary = true;
                db.insert_email(&email).unwrap();

                let mut phone = Phone::new(p.id, "(555) 123-4567".to_string());
                phone.phone_type = PhoneType::Mobile;
                phone.is_primary = true;
                db.insert_phone(&phone).unwrap();

                let mut addr = Address::new(p.id);
                addr.city = Some("Austin".to_string());
                addr.state = Some("TX".to_string());
                addr.address_type = AddressType::Home;
                addr.is_primary = true;
                db.insert_address(&addr).unwrap();
            }
        }

        db
    }

    #[test]
    fn test_show_by_uuid() {
        let db = setup_test_db();

        // Get the UUID of John Smith
        let results = db.search_persons_multi(&["john", "smith"], false, 1).unwrap();
        assert_eq!(results.len(), 1);

        let uuid = results[0].id;
        let detail = db.get_contact_detail(uuid).unwrap();
        assert!(detail.is_some());

        let detail = detail.unwrap();
        assert_eq!(detail.person.name_given, Some("John".to_string()));
        assert_eq!(detail.emails.len(), 1);
        assert_eq!(detail.phones.len(), 1);
        assert_eq!(detail.addresses.len(), 1);
    }

    #[test]
    fn test_search_single_match() {
        let db = setup_test_db();

        // "John Smith" should match exactly one person
        let results = db.search_persons_multi(&["john", "smith"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, Some("John Smith".to_string()));
    }

    #[test]
    fn test_search_multiple_matches() {
        let db = setup_test_db();

        // "Smith" should match two people
        let results = db.search_persons_multi(&["smith"], false, 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_no_match() {
        let db = setup_test_db();

        let results = db.search_persons_multi(&["nonexistent"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_empty_identifier_error() {
        let db = setup_test_db();
        let result = run_show(&db, "   ");
        assert!(result.is_err());
    }
}
