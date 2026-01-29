use anyhow::{anyhow, Result};
use std::path::Path;
use uuid::Uuid;

use crate::cli::photo_utils;
use crate::cli::ui::select_contact;
use crate::db::Database;
use crate::models::Person;

/// Execute the photo command - set or clear a contact's photo
pub fn run_photo(db: &Database, identifier: &str, image_path: Option<&str>, clear: bool) -> Result<()> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Contact identifier cannot be empty."));
    }

    // Find the contact
    let person = find_person(db, identifier)?;
    let Some(person) = person else {
        println!("No contact found.");
        return Ok(());
    };

    let display_name = person.display_name.clone().unwrap_or_else(|| "(unnamed)".to_string());

    if clear {
        // Clear existing photo
        photo_utils::delete_photo(person.id);
        println!("Photo cleared for {}.", display_name);
        return Ok(());
    }

    let Some(image_path) = image_path else {
        // Show current photo status
        if photo_utils::photo_exists(person.id) {
            let path = photo_utils::photo_path(person.id)?;
            println!("{}: {}", display_name, path.display());
        } else {
            println!("{}: no photo", display_name);
        }
        return Ok(());
    };

    // Validate input path exists
    let source_path = Path::new(image_path);
    if !source_path.exists() {
        return Err(anyhow!("Image file not found: {}", image_path));
    }

    // Save photo (validates image, converts to JPEG)
    photo_utils::save_photo(person.id, source_path)?;

    println!("Photo set for {}.", display_name);
    Ok(())
}

fn find_person(db: &Database, identifier: &str) -> Result<Option<Person>> {
    // Try parsing as UUID first
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
            // Multiple matches - let user select
            select_contact(db, &results)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Jane".to_string());
        p.name_family = Some("Doe".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        db
    }

    #[test]
    fn test_find_person_by_name() {
        let db = setup_test_db();
        let person = find_person(&db, "jane").unwrap();
        assert!(person.is_some());
        assert_eq!(person.unwrap().name_given, Some("Jane".to_string()));
    }

    #[test]
    fn test_empty_identifier_error() {
        let db = setup_test_db();
        let result = run_photo(&db, "   ", None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_image_error() {
        let db = setup_test_db();
        let result = run_photo(&db, "jane", Some("/nonexistent/path.jpg"), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_image_error() {
        let db = setup_test_db();
        // Create a temp file with invalid image data
        let mut temp = NamedTempFile::with_suffix(".jpg").unwrap();
        writeln!(temp, "fake image data").unwrap();
        let result = run_photo(&db, "jane", Some(temp.path().to_str().unwrap()), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid image"));
    }
}
