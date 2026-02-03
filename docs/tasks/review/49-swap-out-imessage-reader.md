# Task 49: Swap out iMessage reader

**Feature**: none
**Created**: 2026-01-29
**Depends on**: none
**Blocks**: none

## Problem

The current `attributedBody` blob parser in `src/cli/messages/macos.rs` uses a heuristic approach that scans for ASCII strings and picks the longest one. This fails when serialized metadata (e.g., currency codes like `kUSD-CAD-AUD-HKD-...`, locale identifiers) is longer than the actual message text. A temporary fix was added (preferring strings with spaces, filtering encoded identifiers), but this is fragile.

The proper solution is to use the `imessage-database` crate from the [imessage-exporter](https://github.com/ReagentX/imessage-exporter) project, which has a proper typedstream parser specifically designed for iMessage's `attributedBody` format.

## Success criteria

- [x] Messages display correct text content (no garbled currency codes or binary artifacts)
- [x] Short messages like "Yes" or "Thanks" display correctly
- [x] Messages with attachments show placeholder text (U+FFFC)
- [x] Edited messages are handled gracefully
- [x] All existing message functionality works (search, view, conversation display)

## Completed

### Changes made

1. **Added dependency** (`Cargo.toml`):
   ```toml
   imessage-database = "3.3"
   ```

2. **Removed ~150 lines of heuristic parsing code** (`src/cli/messages/macos.rs`):
   - `NS_CLASS_NAMES` constant
   - `is_attachment_ref()`
   - `extract_text_from_body()` (the main heuristic parser)
   - `looks_like_encoded_identifier()`
   - `is_binary_plist_text()`

3. **Simplified `extract_message_text()`** to use the library:
   ```rust
   use imessage_database::util::streamtyped::parse as parse_typedstream;

   fn extract_message_text(text: Option<String>, attributed_body: Option<Vec<u8>>) -> Option<String> {
       if let Some(ref t) = text {
           if !t.is_empty() {
               return Some(t.clone());
           }
       }

       if let Some(blob) = attributed_body {
           if let Ok(parsed) = parse_typedstream(blob) {
               if !parsed.is_empty() {
                   return Some(parsed);
               }
           }
       }

       None
   }
   ```

4. **Updated tests** - Replaced old heuristic tests with tests for `extract_message_text()`

### Verification

- `cargo build` - Compiles successfully
- `cargo test` - All 78 tests pass

## Notes

### Library details

- **Crate**: [imessage-database](https://crates.io/crates/imessage-database) v3.3.1
- **Docs**: [streamtyped::parse](https://docs.rs/imessage-database/latest/imessage_database/util/streamtyped/fn.parse.html)
- **Technical blog**: [Reverse engineering Apple's typedstream format](https://chrissardegna.com/blog/reverse-engineering-apples-typedstream-format/)

### What the library handles

- Proper typedstream binary format parsing
- NSMutableAttributedString deserialization
- Fallback to legacy string parsing for older formats
- Attachment placeholders (U+FFFC for attachments, U+FFFD for app messages)
- Mention annotations and ranges

### Note on version

Used v3.3 instead of v1 due to rusqlite version conflict (v1 uses rusqlite 0.28, contactcmd uses 0.38).
