# Task 39: Import and sync photos with view option

**Feature**: /docs/features/contacts.md
**Created**: 2026-01-28
**Depends on**: none
**Blocks**: none

## Problem

Contacts are text-only, making it harder to quickly recognize who you're looking at. macOS Contacts already has photos for many contacts. Users should be able to see contact photos when viewing a contact, swap/update them manually, and potentially sync them with other systems. Photos should be stored locally for speed and to support contacts that don't come from macOS Contacts.

## Success criteria

- [x] Photos synced from macOS Contacts during `sync mac` (with prompt: "Sync photos? [y/n]")
- [x] Photos stored as files in `~/.config/contactcmd/photos/{uuid}.{ext}`
- [x] Photo displayed inline when viewing a contact (iTerm2/Kitty/Sixel support)
- [x] Graceful fallback when terminal doesn't support images (no photo shown, no error)
- [x] User can manually add/replace photo via `contactcmd photo <name> <path>`
- [x] Photos cleaned up when contact is deleted
- [x] Photos NOT added to git (stored outside repo in ~/.config)

## Notes

### Storage approach
- File-based storage keeps database fast and allows manual editing
- Directory: `~/.config/contactcmd/photos/`
- Filename: `{person_uuid}.jpg` (convert other formats to JPEG)
- Add `photo_path` column to persons table (nullable, stores relative path)
- Photos outside git repo by design; .gitignore updated as safety net

### Fetching from macOS Contacts
Use `objc2_contacts` (same as existing sync) with:
- `CNContactImageDataKey` - full resolution photo
- `CNContactThumbnailImageDataKey` - smaller, faster option

**Why not SQLite/ZABCDLIKENESS:**
- `objc2_contacts` is the official API (won't break with macOS updates)
- Consistent with existing sync code
- Respects system permissions properly

### Terminal image display
Use `viuer` crate (v0.11.0, updated Dec 2025):
- Auto-detects iTerm2/Kitty/Sixel/halfblocks
- Simple API: `viuer::print_from_file(path)`
- Falls back gracefully if terminal doesn't support images

### Implementation steps

#### 1. Add dependencies to Cargo.toml
```toml
viuer = "0.11"
image = "0.25"  # for format conversion
```

#### 2. Schema migration (src/db/schema.rs)
```rust
pub const SCHEMA_VERSION: i32 = 2;

pub const MIGRATION_V2: &str = r#"
ALTER TABLE persons ADD COLUMN photo_path TEXT;
"#;
```
Update `run_migrations()` in `src/db/mod.rs` to apply V2.

#### 3. Photo directory
In `Database::open()`, create `~/.config/contactcmd/photos/` if it doesn't exist.

#### 4. Sync photos (src/cli/sync/macos.rs)
Add to fetch keys:
```rust
use objc2_contacts::CNContactImageDataKey;
// Add to keys_to_fetch array
```
After syncing contacts, prompt: `"Sync photos? [y/n]: "`
If yes, iterate contacts with `imageData`, save to `photos/{uuid}.jpg`.

#### 5. Display (src/cli/display.rs)
```rust
use viuer::{Config, print_from_file};

// Before printing contact name:
if let Some(photo_path) = &detail.person.photo_path {
    let conf = Config {
        width: Some(20),  // ~20 chars wide
        height: Some(10), // ~10 lines tall
        ..Default::default()
    };
    let _ = print_from_file(photo_path, &conf); // ignore errors
}
```

#### 6. Manual add command (src/cli/photo.rs)
```
contactcmd photo "John Smith" ~/Pictures/john.jpg
```
Copy file to `~/.config/contactcmd/photos/{uuid}.jpg`, update `photo_path` in DB.

#### 7. Cleanup (src/cli/delete.rs)
When deleting contact, also delete `photos/{uuid}.jpg` if exists.

### Edge cases
- Contact has no photo in macOS: skip, no placeholder
- Photo format is PNG/HEIC: convert to JPEG on save (use `image` crate)
- Terminal doesn't support images: viuer handles gracefully
- Photo file missing but path set: show contact without photo, log warning
- Contacts not from macOS: can still add photos manually
- User declines photo sync: only sync contact data, not photos

## Files touched

**New files:**
- `src/cli/photo.rs` - Manual photo command implementation

**Modified files:**
- `Cargo.toml` - Added viuer, image dependencies; tempfile dev-dependency
- `src/db/schema.rs` - Added SCHEMA_VERSION=2, MIGRATION_V2 for photo_path column
- `src/db/mod.rs` - Added photos_dir(), photos directory creation, V2 migration logic, updated test
- `src/db/persons.rs` - Added photo_path to insert_person, update_person, row_to_person
- `src/models/person.rs` - Added photo_path field to Person struct
- `src/cli/display.rs` - Added display_photo() function using viuer
- `src/cli/delete.rs` - Added delete_photo_file() for cleanup on contact delete
- `src/cli/mod.rs` - Added photo module, PhotoArgs struct, Photo command variant
- `src/main.rs` - Added photo command handler
- `src/cli/sync/macos.rs` - Added CNContactImageDataKey to fetch, photo sync with prompt

## Remaining work

All items complete. Added:
- `CNContactImageDataKey` to fetch keys in `src/cli/sync/macos.rs`
- `should_sync_photos()` function with y/N prompt
- `sync_photos()` function that iterates contacts, saves image data to photos directory
- `detect_image_format()` function to detect PNG vs JPEG from magic bytes
