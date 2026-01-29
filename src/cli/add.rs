use anyhow::{anyhow, Result};
use inquire::Confirm;

use crate::cli::ui::{is_valid_email, minimal_render_config, prompt_field_optional, FormResult};
use crate::db::Database;
use crate::models::{Email, Person, Phone};

/// Execute the add command
pub fn run_add(
    db: &Database,
    first: Option<String>,
    last: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    notes: Option<String>,
) -> Result<()> {
    // If no options provided, run interactive mode
    let all_none = first.is_none() && last.is_none() && email.is_none() && phone.is_none() && notes.is_none();
    let (first, last, email, phone, notes) = if all_none {
        interactive_mode()?
    } else {
        (first, last, email, phone, notes)
    };

    // Validate: at least first or last name required
    if first.is_none() && last.is_none() {
        return Err(anyhow!("At least first or last name is required."));
    }

    // Validate email format if provided
    if let Some(ref e) = email {
        if !is_valid_email(e) {
            return Err(anyhow!("Invalid email format: {}", e));
        }
    }

    // Check for duplicates
    if let Some(duplicate) = check_duplicate(db, &first, &last, &email)? {
        println!("Warning: Similar contact exists:");
        println!("  {}", duplicate);
        println!();

        let confirmed = Confirm::new("Continue anyway?")
            .with_render_config(minimal_render_config())
            .with_default(false)
            .prompt()
            .unwrap_or(false);

        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Create person
    let mut person = Person::new();
    person.name_given = first;
    person.name_family = last;
    person.notes = notes;
    person.compute_names();

    db.insert_person(&person)?;

    // Add email if provided
    if let Some(email_addr) = email {
        let mut email_record = Email::new(person.id, email_addr);
        email_record.is_primary = true;
        db.insert_email(&email_record)?;
    }

    // Add phone if provided
    if let Some(phone_num) = phone {
        let mut phone_record = Phone::new(person.id, phone_num);
        phone_record.is_primary = true;
        db.insert_phone(&phone_record)?;
    }

    let display_name = person.display_name.as_deref().unwrap_or("(unnamed)");
    println!("\nCreated: {}", display_name);

    Ok(())
}

fn interactive_mode() -> Result<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> {
    let first = match prompt_field_optional("first")? {
        FormResult::Value(v) => if v.is_empty() { None } else { Some(v) },
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok((None, None, None, None, None));
        }
    };

    let last = match prompt_field_optional("last")? {
        FormResult::Value(v) => if v.is_empty() { None } else { Some(v) },
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok((None, None, None, None, None));
        }
    };

    let email = match prompt_field_optional("email")? {
        FormResult::Value(v) => if v.is_empty() { None } else { Some(v) },
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok((None, None, None, None, None));
        }
    };

    let phone = match prompt_field_optional("phone")? {
        FormResult::Value(v) => if v.is_empty() { None } else { Some(v) },
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok((None, None, None, None, None));
        }
    };

    let notes = match prompt_field_optional("notes")? {
        FormResult::Value(v) => if v.is_empty() { None } else { Some(v) },
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok((None, None, None, None, None));
        }
    };

    Ok((first, last, email, phone, notes))
}

fn check_duplicate(
    db: &Database,
    first: &Option<String>,
    last: &Option<String>,
    email: &Option<String>,
) -> Result<Option<String>> {
    // Build search terms from name
    let mut words = Vec::new();
    if let Some(ref f) = first {
        words.push(f.as_str());
    }
    if let Some(ref l) = last {
        words.push(l.as_str());
    }

    if words.is_empty() {
        return Ok(None);
    }

    let results = db.search_persons_multi(&words, false, 5)?;

    for person in results {
        // Check if name matches closely
        let name_match = person.name_given.as_ref() == first.as_ref()
            && person.name_family.as_ref() == last.as_ref();

        if name_match {
            let display = person.display_name.unwrap_or_else(|| "(unnamed)".to_string());
            let emails = db.get_emails_for_person(person.id)?;
            let email_str = emails.first().map(|e| e.email_address.as_str()).unwrap_or("");

            if !email_str.is_empty() {
                return Ok(Some(format!("{} ({})", display, email_str)));
            } else {
                return Ok(Some(display));
            }
        }

        // Also check email match if provided
        if let Some(ref new_email) = email {
            let emails = db.get_emails_for_person(person.id)?;
            for e in emails {
                if e.email_address.eq_ignore_ascii_case(new_email) {
                    let display = person.display_name.unwrap_or_else(|| "(unnamed)".to_string());
                    return Ok(Some(format!("{} ({})", display, e.email_address)));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.co.uk"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@domain"));
    }

    #[test]
    fn test_add_person_direct() {
        let db = Database::open_memory().unwrap();

        run_add(
            &db,
            Some("John".to_string()),
            Some("Smith".to_string()),
            Some("john@example.com".to_string()),
            Some("555-1234".to_string()),
            Some("Test contact".to_string()),
        ).unwrap();

        let results = db.search_persons_multi(&["john", "smith"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("John".to_string()));
        assert_eq!(results[0].name_family, Some("Smith".to_string()));
        assert_eq!(results[0].notes, Some("Test contact".to_string()));

        let emails = db.get_emails_for_person(results[0].id).unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].email_address, "john@example.com");

        let phones = db.get_phones_for_person(results[0].id).unwrap();
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0].phone_number, "555-1234");
    }

    #[test]
    fn test_add_first_name_only() {
        let db = Database::open_memory().unwrap();

        run_add(&db, Some("Alice".to_string()), None, None, None, None).unwrap();

        let results = db.search_persons_multi(&["alice"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_add_requires_name() {
        let db = Database::open_memory().unwrap();

        let result = run_add(&db, None, None, Some("test@example.com".to_string()), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_invalid_email() {
        let db = Database::open_memory().unwrap();

        let result = run_add(
            &db,
            Some("John".to_string()),
            None,
            Some("invalid-email".to_string()),
            None,
            None,
        );
        assert!(result.is_err());
    }
}
