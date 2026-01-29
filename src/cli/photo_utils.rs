use anyhow::{anyhow, Result};
use image::ImageFormat;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::db::Database;

/// Get the canonical photo path for a person (always .jpg)
pub fn photo_path(person_id: Uuid) -> Result<PathBuf> {
    let photos_dir = Database::photos_dir()?;
    Ok(photos_dir.join(format!("{}.jpg", person_id)))
}

/// Check if a photo exists for a person
pub fn photo_exists(person_id: Uuid) -> bool {
    photo_path(person_id)
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Delete a person's photo if it exists
pub fn delete_photo(person_id: Uuid) {
    if let Ok(path) = photo_path(person_id) {
        let _ = std::fs::remove_file(path);
    }
}

/// Save a photo from a source file, validating and converting to JPEG
pub fn save_photo(person_id: Uuid, source: &Path) -> Result<()> {
    // Read and validate the image
    let img = image::open(source)
        .map_err(|e| anyhow!("Invalid image file: {}", e))?;

    // Save as JPEG
    let dest = photo_path(person_id)?;
    img.save_with_format(&dest, ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to save photo: {}", e))?;

    Ok(())
}

/// Save a photo from raw bytes (for sync), validating and converting to JPEG
pub fn save_photo_bytes(person_id: Uuid, bytes: &[u8]) -> Result<()> {
    // Validate and decode the image
    let img = image::load_from_memory(bytes)
        .map_err(|e| anyhow!("Invalid image data: {}", e))?;

    // Save as JPEG
    let dest = photo_path(person_id)?;
    img.save_with_format(&dest, ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to save photo: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_photo_path_format() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let path = photo_path(id).unwrap();
        assert!(path.to_string_lossy().ends_with("550e8400-e29b-41d4-a716-446655440000.jpg"));
    }

    #[test]
    fn test_photo_exists_false_for_nonexistent() {
        let id = Uuid::new_v4();
        assert!(!photo_exists(id));
    }

    #[test]
    fn test_delete_photo_no_panic_when_missing() {
        let id = Uuid::new_v4();
        delete_photo(id); // Should not panic
    }

    #[test]
    fn test_save_photo_invalid_file() {
        let id = Uuid::new_v4();

        // Create a temp file with invalid image data
        let mut temp = NamedTempFile::with_suffix(".jpg").unwrap();
        writeln!(temp, "not an image").unwrap();

        let result = save_photo(id, temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_save_photo_bytes_invalid() {
        let id = Uuid::new_v4();
        let result = save_photo_bytes(id, b"not an image");
        assert!(result.is_err());
    }
}
