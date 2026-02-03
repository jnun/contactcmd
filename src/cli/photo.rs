use anyhow::{anyhow, Result};
use rfd::FileDialog;
use std::path::Path;

use crate::cli::photo_utils;
use crate::cli::ui::{find_person_by_identifier, get_display_name};
use crate::db::Database;

/// Opens a native file picker dialog to select an image file.
/// Returns `None` if the user cancels the dialog.
fn pick_image_file() -> Option<String> {
    FileDialog::new()
        .add_filter("Images", &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff"])
        .set_title("Select photo for contact")
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

/// Execute the photo command - set or clear a contact's photo
pub fn run_photo(db: &Database, identifier: &str, image_path: Option<&str>, clear: bool) -> Result<()> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Contact identifier cannot be empty."));
    }

    // Find the contact
    let Some(person) = find_person_by_identifier(db, identifier)? else {
        println!("No contact found.");
        return Ok(());
    };

    let display_name = get_display_name(&person);

    if clear {
        // Clear existing photo
        photo_utils::delete_photo(person.id);
        println!("Photo cleared for {}.", display_name);
        return Ok(());
    }

    // Get image path from argument or file picker
    let image_path = match image_path {
        Some(p) => p.to_string(),
        None => {
            // Open native file picker
            match pick_image_file() {
                Some(p) => p,
                None => {
                    println!("No file selected.");
                    return Ok(());
                }
            }
        }
    };

    // Validate input path exists
    let source_path = Path::new(&image_path);
    if !source_path.exists() {
        return Err(anyhow!("Image file not found: {}", image_path));
    }

    // Save photo (validates image, converts to JPEG)
    photo_utils::save_photo(person.id, source_path)?;

    println!("Photo set for {}.", display_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Person;
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
        let person = find_person_by_identifier(&db, "jane").unwrap();
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
