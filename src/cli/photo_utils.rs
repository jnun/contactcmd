use anyhow::{anyhow, Result};
use image::ImageFormat;
use std::fs;
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

/// Delete a person's photo and hash sidecar if they exist
pub fn delete_photo(person_id: Uuid) {
    if let Ok(path) = photo_path(person_id) {
        let _ = fs::remove_file(path);
    }
    if let Ok(path) = hash_path(person_id) {
        let _ = fs::remove_file(path);
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

/// Result of attempting to save a photo with change detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveResult {
    /// Photo was saved (new or changed)
    Saved,
    /// Photo unchanged, skipped
    Unchanged,
    /// Invalid image data
    Invalid,
}

/// Get the path to the hash sidecar file for a photo
fn hash_path(person_id: Uuid) -> Result<PathBuf> {
    let photos_dir = Database::photos_dir()?;
    Ok(photos_dir.join(format!("{}.hash", person_id)))
}

/// Read stored source hash from sidecar file
fn read_stored_hash(person_id: Uuid) -> Option<u64> {
    let path = hash_path(person_id).ok()?;
    let contents = fs::read_to_string(&path).ok()?;
    contents.trim().parse().ok()
}

/// Write source hash to sidecar file
fn write_stored_hash(person_id: Uuid, hash: u64) {
    if let Ok(path) = hash_path(person_id) {
        let _ = fs::write(&path, hash.to_string());
    }
}

/// Save a photo from raw bytes only if it differs from existing file.
/// Returns whether the photo was saved, unchanged, or invalid.
///
/// Uses a sidecar .hash file to store the hash of the original source bytes,
/// allowing accurate change detection across syncs (source format may differ
/// from stored JPEG format).
pub fn save_photo_bytes_if_changed(person_id: Uuid, bytes: &[u8]) -> SaveResult {
    // Validate and decode the image first
    let img = match image::load_from_memory(bytes) {
        Ok(img) => img,
        Err(_) => return SaveResult::Invalid,
    };

    let dest = match photo_path(person_id) {
        Ok(p) => p,
        Err(_) => return SaveResult::Invalid,
    };

    // Compare source hash against stored source hash (not the JPEG output)
    let source_hash = simple_hash(bytes);
    if dest.exists() {
        if let Some(stored_hash) = read_stored_hash(person_id) {
            if source_hash == stored_hash {
                return SaveResult::Unchanged;
            }
        }
    }

    // Save as JPEG
    match img.save_with_format(&dest, ImageFormat::Jpeg) {
        Ok(_) => {
            write_stored_hash(person_id, source_hash);
            SaveResult::Saved
        }
        Err(_) => SaveResult::Invalid,
    }
}

/// Simple hash for comparing image data (FNV-1a)
fn simple_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Check if saving would result in a change (for dry-run mode).
/// Does not write anything to disk.
pub fn would_photo_change(person_id: Uuid, bytes: &[u8]) -> SaveResult {
    // Validate the image first
    if image::load_from_memory(bytes).is_err() {
        return SaveResult::Invalid;
    }

    let dest = match photo_path(person_id) {
        Ok(p) => p,
        Err(_) => return SaveResult::Invalid,
    };

    // If no existing file, it would be saved
    if !dest.exists() {
        return SaveResult::Saved;
    }

    // Compare source hash against stored source hash
    let source_hash = simple_hash(bytes);
    if let Some(stored_hash) = read_stored_hash(person_id) {
        if source_hash == stored_hash {
            return SaveResult::Unchanged;
        }
    }

    SaveResult::Saved
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

    #[test]
    fn test_save_result_enum() {
        // Just verify the enum variants exist and can be compared
        assert_eq!(SaveResult::Saved, SaveResult::Saved);
        assert_eq!(SaveResult::Unchanged, SaveResult::Unchanged);
        assert_eq!(SaveResult::Invalid, SaveResult::Invalid);
        assert_ne!(SaveResult::Saved, SaveResult::Unchanged);
    }

    #[test]
    fn test_would_photo_change_invalid_data() {
        let id = Uuid::new_v4();
        let result = would_photo_change(id, b"not an image");
        assert_eq!(result, SaveResult::Invalid);
    }

    #[test]
    fn test_save_photo_bytes_if_changed_invalid() {
        let id = Uuid::new_v4();
        let result = save_photo_bytes_if_changed(id, b"not an image");
        assert_eq!(result, SaveResult::Invalid);
    }

    #[test]
    fn test_simple_hash_consistency() {
        let data = b"test data for hashing";
        let hash1 = simple_hash(data);
        let hash2 = simple_hash(data);
        assert_eq!(hash1, hash2);

        let different = b"different data";
        let hash3 = simple_hash(different);
        assert_ne!(hash1, hash3);
    }
}
