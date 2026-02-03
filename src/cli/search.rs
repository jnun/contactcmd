use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::io::{self, Write};

use crate::db::Database;
use crate::models::Person;
use super::display::{print_full_contact, print_full_contact_with_tasks};
use super::list::{handle_full_edit, handle_notes};
use super::messages::get_last_message_for_handles;
use super::show::show_messages_screen;
use super::task::run_tasks_for_contact;
use super::ui::{clear_screen, confirm, prompt_undo, show_help, task_action_label, RawModeGuard, StatusBar};
#[cfg(target_os = "macos")]
use super::sync::{delete_from_macos_contacts, get_apple_id};

/// Parsed natural language query
/// Supports: "jason in alabama" or "jason at google" or "jason smith in tx at acme"
#[derive(Debug, Default)]
struct ParsedQuery {
    name_terms: Vec<String>,
    location_terms: Vec<String>,
    org_terms: Vec<String>,
}

impl ParsedQuery {
    /// Check if this is a natural language query (contains " in " or " at ")
    fn is_natural_query(query: &str) -> bool {
        let lower = query.to_lowercase();
        lower.contains(" in ") || lower.contains(" at ")
    }

    /// Parse a natural language query like "jason in alabama at google"
    /// Keywords: "in" for location, "at" for organization
    fn parse(query: &str) -> Self {
        let mut result = ParsedQuery::default();

        // We need to find " in " and " at " as word boundaries
        // Strategy: scan through and split on these keywords
        let lower = query.to_lowercase();
        let chars: Vec<char> = query.chars().collect();
        let lower_chars: Vec<char> = lower.chars().collect();

        // Find positions of " in " and " at "
        let mut segments: Vec<(usize, &str)> = vec![(0, "name")]; // (position, type)

        let mut i = 0;
        while i < lower_chars.len() {
            // Check for " in " (4 chars)
            if i + 4 <= lower_chars.len() {
                let slice: String = lower_chars[i..i+4].iter().collect();
                if slice == " in " {
                    segments.push((i + 4, "location"));
                    i += 4;
                    continue;
                }
            }
            // Check for " at " (4 chars)
            if i + 4 <= lower_chars.len() {
                let slice: String = lower_chars[i..i+4].iter().collect();
                if slice == " at " {
                    segments.push((i + 4, "org"));
                    i += 4;
                    continue;
                }
            }
            i += 1;
        }

        // Extract text for each segment
        for (idx, &(start, segment_type)) in segments.iter().enumerate() {
            // Find end: either next segment start - 4 (for the keyword), or end of string
            let end = if idx + 1 < segments.len() {
                // Go back 4 chars to exclude the " in " or " at " keyword
                segments[idx + 1].0 - 4
            } else {
                chars.len()
            };

            let segment_text: String = chars[start..end].iter().collect();
            let words: Vec<String> = segment_text
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            match segment_type {
                "name" => result.name_terms = words,
                "location" => result.location_terms.extend(words),
                "org" => result.org_terms.extend(words),
                _ => {}
            }
        }

        result
    }

    /// Check if the query has any terms
    fn is_empty(&self) -> bool {
        self.name_terms.is_empty() && self.location_terms.is_empty() && self.org_terms.is_empty()
    }
}

/// Execute the search command
pub fn run_search(db: &Database, query: &str, case_sensitive: bool, missing: Option<&str>, field: Option<&str>) -> Result<()> {
    // Handle --missing flag (CLI usage)
    if let Some(field) = missing {
        return run_missing_search(db, field);
    }

    let query = query.trim();
    if query.is_empty() {
        println!("No query.");
        return Ok(());
    }

    // Handle special syntax from menu
    if query == "email:missing" {
        return run_missing_search(db, "email");
    }
    if query == "phone:missing" {
        return run_missing_search(db, "phone");
    }

    // Parse field prefix
    let (search_field, clean_query) = if let Some(rest) = query.strip_prefix('@') {
        (Some("name"), rest.trim())
    } else if let Some(rest) = query.strip_prefix('#') {
        (Some("city"), rest.trim())
    } else {
        (field, query)
    };

    if clean_query.is_empty() {
        println!("No query.");
        return Ok(());
    }

    // Check for natural language query (contains " in " or " at ")
    let results = if search_field.is_none() && ParsedQuery::is_natural_query(clean_query) {
        let parsed = ParsedQuery::parse(clean_query);
        if parsed.is_empty() {
            println!("No query.");
            return Ok(());
        }
        let name_refs: Vec<&str> = parsed.name_terms.iter().map(|s| s.as_str()).collect();
        let loc_refs: Vec<&str> = parsed.location_terms.iter().map(|s| s.as_str()).collect();
        let org_refs: Vec<&str> = parsed.org_terms.iter().map(|s| s.as_str()).collect();
        db.search_persons_natural(&name_refs, &loc_refs, &org_refs, case_sensitive, u32::MAX)?
    } else {
        let words: Vec<&str> = clean_query.split_whitespace().collect();
        if let Some(f) = search_field {
            db.search_persons_by_field(&words, f, case_sensitive, u32::MAX)?
        } else {
            db.search_persons_multi(&words, case_sensitive, u32::MAX)?
        }
    };

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

        // Fetch pending tasks for preview (limit to 3)
        let pending_tasks = db.get_pending_tasks_for_person(detail.person.id, 3)?;
        print_full_contact_with_tasks(&detail, last_message.as_ref(), &pending_tasks);

        // Use the count for the status bar label
        let pending_count = pending_tasks.len() as u32;

        let status = StatusBar::new()
            .counter(index + 1, results.len())
            .action("e", "dit")
            .action("m", "sg")
            .action("n", "ote")
            .action("t", &task_action_label(pending_count))
            .action("d", "el")
            .separator()
            .action("?", "")
            .action("q", "")
            .action("Q", "uit")
            .render();
        print!("\n{}: ", status);
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
                handle_full_edit(db, &detail)?;
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
            KeyCode::Char('t') | KeyCode::Char('T') => {
                println!();
                let person_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                if run_tasks_for_contact(db, detail.person.id, person_name)? {
                    break; // Quit requested from tasks screen
                }
                // Continue showing same contact after returning from tasks
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                println!();

                // Confirm before delete
                if !confirm(&format!("Delete \"{}\"?", display_name))? {
                    continue;
                }

                // Backup before delete for undo
                let backup = detail.clone();

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
                    // Show undo prompt with 5 second timeout
                    if prompt_undo(&format!("Deleted \"{}\"", display_name), 5)? {
                        db.restore_person(&backup)?;
                        println!("Restored.");
                        // Stay on same contact - don't advance index
                    } else {
                        // User didn't undo - advance to next
                        index += 1;
                    }
                }
            }
            // Navigation: next contact
            KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Enter | KeyCode::Char(' ') => {
                index += 1;
            }
            // Navigation: previous contact
            KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('K') => {
                index = index.saturating_sub(1);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Back to menu
                break;
            }
            KeyCode::Char('Q') => {
                // Quit app entirely
                clear_screen()?;
                std::process::exit(0);
            }
            KeyCode::Char('?') => {
                show_help("search")?;
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
    use crate::models::{Address, Email, Organization, PersonOrganization};

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
        p.notes = Some("Met at conference in Berlin".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

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
        p.notes = Some("Definitely not Batman".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut addr = Address::new(p.id);
        addr.city = Some("Gotham".to_string());
        db.insert_address(&addr).unwrap();

        let org = Organization::new("Wayne Enterprises".to_string());
        db.insert_organization(&org).unwrap();
        let po = PersonOrganization::new(p.id, org.id);
        db.insert_person_organization(&po).unwrap();

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

    #[test]
    fn test_search_by_field_note() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Sarah".to_string());
        p.name_family = Some("Connor".to_string());
        p.notes = Some("Met at conference in Berlin".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        // Search by note field
        let results = db.search_persons_by_field(&["conference"], "note", false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("Sarah".to_string()));

        // Search for word not in note but in name - should NOT match
        let results = db.search_persons_by_field(&["sarah"], "note", false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_field_state() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Bob".to_string());
        p.name_family = Some("Texas".to_string()); // Last name is Texas
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut addr = Address::new(p.id);
        addr.state = Some("TX".to_string());
        db.insert_address(&addr).unwrap();

        // Search by state field - should find by state
        let results = db.search_persons_by_field(&["TX"], "state", false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // Search for name "Texas" in state field - should NOT match
        let results = db.search_persons_by_field(&["Texas"], "state", false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    // =====================
    // Natural Query Parser Tests
    // =====================

    #[test]
    fn test_parse_simple_in() {
        let parsed = ParsedQuery::parse("jason in alabama");
        assert_eq!(parsed.name_terms, vec!["jason"]);
        assert_eq!(parsed.location_terms, vec!["alabama"]);
        assert!(parsed.org_terms.is_empty());
    }

    #[test]
    fn test_parse_simple_at() {
        let parsed = ParsedQuery::parse("jason at google");
        assert_eq!(parsed.name_terms, vec!["jason"]);
        assert!(parsed.location_terms.is_empty());
        assert_eq!(parsed.org_terms, vec!["google"]);
    }

    #[test]
    fn test_parse_multi_word_name() {
        let parsed = ParsedQuery::parse("jason smith in alabama");
        assert_eq!(parsed.name_terms, vec!["jason", "smith"]);
        assert_eq!(parsed.location_terms, vec!["alabama"]);
    }

    #[test]
    fn test_parse_both_in_and_at() {
        let parsed = ParsedQuery::parse("jason in tx at acme");
        assert_eq!(parsed.name_terms, vec!["jason"]);
        assert_eq!(parsed.location_terms, vec!["tx"]);
        assert_eq!(parsed.org_terms, vec!["acme"]);
    }

    #[test]
    fn test_parse_at_before_in() {
        let parsed = ParsedQuery::parse("jason at google in seattle");
        assert_eq!(parsed.name_terms, vec!["jason"]);
        assert_eq!(parsed.location_terms, vec!["seattle"]);
        assert_eq!(parsed.org_terms, vec!["google"]);
    }

    #[test]
    fn test_parse_multi_word_location() {
        let parsed = ParsedQuery::parse("jason in new york");
        assert_eq!(parsed.name_terms, vec!["jason"]);
        assert_eq!(parsed.location_terms, vec!["new", "york"]);
    }

    #[test]
    fn test_parse_preserves_case() {
        let parsed = ParsedQuery::parse("Jason IN Alabama");
        assert_eq!(parsed.name_terms, vec!["Jason"]);
        assert_eq!(parsed.location_terms, vec!["Alabama"]);
    }

    #[test]
    fn test_is_natural_query() {
        assert!(ParsedQuery::is_natural_query("jason in alabama"));
        assert!(ParsedQuery::is_natural_query("jason at google"));
        assert!(ParsedQuery::is_natural_query("Jason IN Texas"));
        assert!(!ParsedQuery::is_natural_query("jason smith"));
        assert!(!ParsedQuery::is_natural_query("katrina")); // "in" within word
        assert!(!ParsedQuery::is_natural_query("patrick")); // "at" within word
    }

    #[test]
    fn test_no_false_match_within_words() {
        // "katrina" contains "in", "patrick" contains "at" - should NOT trigger
        assert!(!ParsedQuery::is_natural_query("katrina"));
        assert!(!ParsedQuery::is_natural_query("patrick"));
        // But "kat in alabama" should
        assert!(ParsedQuery::is_natural_query("kat in alabama"));
    }

    // =====================
    // Natural Search Integration Tests
    // =====================

    #[test]
    fn test_natural_search_name_and_location() {
        let db = Database::open_memory().unwrap();

        // Create Jason in Alabama
        let mut p1 = Person::new();
        p1.name_given = Some("Jason".to_string());
        p1.name_family = Some("Smith".to_string());
        p1.compute_names();
        db.insert_person(&p1).unwrap();

        let mut addr1 = Address::new(p1.id);
        addr1.state = Some("AL".to_string());
        addr1.city = Some("Birmingham".to_string());
        db.insert_address(&addr1).unwrap();

        // Create Jason in Texas
        let mut p2 = Person::new();
        p2.name_given = Some("Jason".to_string());
        p2.name_family = Some("Doe".to_string());
        p2.compute_names();
        db.insert_person(&p2).unwrap();

        let mut addr2 = Address::new(p2.id);
        addr2.state = Some("TX".to_string());
        db.insert_address(&addr2).unwrap();

        // "jason in al" should find only Alabama Jason
        let results = db.search_persons_natural(&["jason"], &["al"], &[], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_family, Some("Smith".to_string()));

        // "jason in birmingham" also works
        let results = db.search_persons_natural(&["jason"], &["birmingham"], &[], false, 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_natural_search_name_and_org() {
        let db = Database::open_memory().unwrap();

        // Create Jason at Google
        let mut p1 = Person::new();
        p1.name_given = Some("Jason".to_string());
        p1.name_family = Some("Google".to_string()); // Tricky: name contains "Google"
        p1.compute_names();
        db.insert_person(&p1).unwrap();

        let org1 = Organization::new("Acme Corp".to_string());
        db.insert_organization(&org1).unwrap();
        let po1 = PersonOrganization::new(p1.id, org1.id);
        db.insert_person_organization(&po1).unwrap();

        // Create different Jason at Google
        let mut p2 = Person::new();
        p2.name_given = Some("Jason".to_string());
        p2.name_family = Some("Doe".to_string());
        p2.compute_names();
        db.insert_person(&p2).unwrap();

        let org2 = Organization::new("Google".to_string());
        db.insert_organization(&org2).unwrap();
        let po2 = PersonOrganization::new(p2.id, org2.id);
        db.insert_person_organization(&po2).unwrap();

        // "jason at google" should find only the one at Google org
        let results = db.search_persons_natural(&["jason"], &[], &["google"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_family, Some("Doe".to_string()));
    }

    #[test]
    fn test_natural_search_all_three() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Jason".to_string());
        p.name_family = Some("Smith".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut addr = Address::new(p.id);
        addr.city = Some("Austin".to_string());
        addr.state = Some("TX".to_string());
        db.insert_address(&addr).unwrap();

        let org = Organization::new("Acme".to_string());
        db.insert_organization(&org).unwrap();
        let po = PersonOrganization::new(p.id, org.id);
        db.insert_person_organization(&po).unwrap();

        // "jason in tx at acme" should match
        let results = db.search_persons_natural(&["jason"], &["tx"], &["acme"], false, 10).unwrap();
        assert_eq!(results.len(), 1);

        // "jason in tx at google" should NOT match (wrong org)
        let results = db.search_persons_natural(&["jason"], &["tx"], &["google"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
