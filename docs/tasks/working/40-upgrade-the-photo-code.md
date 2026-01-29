# Task 40: Upgrade the photo code

**Feature**: /docs/features/contacts.md
**Created**: 2026-01-28
**Depends on**: Task 39 (completed)
**Blocks**: none

## Problem

The photo feature (Task 39) is functional but has code duplication, missing validation, and efficiency issues that should be addressed for maintainability and robustness.

## Success criteria

- [ ] No duplicated photo utility code across files
- [ ] Photo command validates image content (not just extension)
- [ ] Photo sync skips unchanged files
- [ ] Photo sync has dry-run mode
- [ ] Test coverage for photo sync and format detection

## Issues identified

### 1. Code duplication

**`delete_photo_file` is duplicated**
- Identical function in `src/cli/photo.rs:106-124` and `src/cli/delete.rs:94-112`
- Should be extracted to shared module

**Path resolution logic repeated 3x**
- `display.rs:15-22`, `photo.rs:113-120`, `delete.rs:101-108` all have:
```rust
let full_path = if Path::new(path).is_absolute() {
    path.to_string()
} else {
    match Database::photos_dir() {
        Ok(dir) => dir.join(path).to_string_lossy().to_string(),
        Err(_) => return,
    }
};
```
- Extract to `resolve_photo_path(path: &str) -> Option<PathBuf>`

### 2. Missing validation

**Photo command doesn't validate image content**
- `photo.rs:59` only checks file extension
- User could rename `secrets.txt` to `secrets.jpg` and it's accepted
- Use `image` crate to validate before copying

**No file size limit**
- Could copy 500MB image with no warning
- Consider max size warning (e.g., 10MB)

### 3. Missing format support

**HEIC not supported in manual add**
- `photo.rs:59` allows `["jpg", "jpeg", "png", "gif", "webp"]`
- macOS photos often use HEIC
- **Solution:** Convert HEIC to JPG on import using macOS `sips` tool (no extra Rust deps)
- viuer/image crate don't support HEIC natively; contact photos don't need lossless quality

**detect_image_format is minimal**
- `macos.rs:696-707` only checks PNG/JPEG magic bytes
- Could also detect WebP (`RIFF....WEBP`), GIF (`GIF89a`/`GIF87a`)

### 4. Performance issues

**Photo sync does O(n) database lookups**
- `macos.rs:649` calls `find_person_by_external_id` per contact
- 1000 contacts = 1000 queries
- Batch: fetch all persons with external_ids into HashMap first

**Always overwrites existing photos**
- `macos.rs:671` always writes even if unchanged
- Compare file size/hash, skip if identical

**Full resolution photos stored**
- Could resize to max 512x512 or use `CNContactThumbnailImageDataKey`

### 5. Configuration limitations

**Display dimensions hardcoded**
- `display.rs:29-30` fixed at `width: 24, height: 12`
- Could adapt to terminal size or be configurable

**No dry-run for photo sync**
- Contact sync has `--dry-run`, photo sync doesn't

### 6. Error handling

**Silent failures in display_photo**
- `display.rs:35` ignores all errors
- Could log to stderr in verbose mode

**No feedback when photo file missing**
- If `photo_path` set but file gone, user sees nothing
- Could print `[photo not found]`

### 7. Test coverage gaps

Missing tests for:
- `sync_photos()` function
- `detect_image_format()` function
- `should_sync_photos()` function
- Photo display with missing file
- Photo command with invalid image content

### 8. Minor: String allocations

**Unnecessary String conversions**
- Path resolution creates `String` via `to_string_lossy().to_string()`
- Could keep as `PathBuf` throughout

## Suggested priority

1. **Extract shared code** - Create shared photo utilities module
2. **Validate image content** - Use `image` crate before copying
3. **Batch photo sync lookups** - Pre-fetch persons into HashMap
4. **Skip unchanged photos** - Compare before overwriting
5. **Add dry-run mode** - Show what would sync
6. **Add test coverage** - Sync and format detection

## Files to modify

- `src/cli/photo.rs` - Extract shared code, add validation
- `src/cli/delete.rs` - Use shared utilities
- `src/cli/display.rs` - Use shared path resolution
- `src/cli/sync/macos.rs` - Batch lookups, skip unchanged, dry-run
- New: `src/cli/photo_utils.rs` - Shared photo utilities
