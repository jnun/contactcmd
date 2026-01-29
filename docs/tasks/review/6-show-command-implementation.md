# Task 6: Show Command Implementation

**Feature:** /docs/features/cmd-show.md
**Created:** 2026-01-27
**Depends on:** Task 3 (Person/CRUD), Task 4 (display module)

## User Story

As a user, I want to view full details about a contact by name or ID so that I can see all their information (emails, phones, addresses, notes, etc.) without having to remember their UUID.

## Acceptance Criteria

- [ ] `contactcmd show <uuid>` displays contact by ID
- [ ] `contactcmd show "name"` searches by name and displays
- [ ] When multiple contacts match a name, show numbered selection menu
- [ ] Selection menu shows: number, name, email, location (data already fetched)
- [ ] Full display includes: basics, contact info, dates, notes, metadata
- [ ] Single query for search (no N+1 when building selection menu)
- [ ] Uses shared `print_full_contact()` from display module
- [ ] `cargo test cli::show` passes
- [ ] `cargo run -- show --help` shows usage

## Blocking Issue

**Display module does not exist.** Task 4 claims to have created `src/cli/display.rs` with `print_full_contact()`, but this file does not exist. Either:
1. Create display module first, or
2. Implement display logic inline in show.rs (not recommended - duplicates code)

## Technical Notes

### Identifier Resolution Flow
1. Try parsing identifier as UUID
2. If valid UUID: fetch by ID directly
3. If not UUID: search by name
   - 0 results → "No contacts found" message
   - 1 result → display full details
   - N results → show selection menu

### Database Methods Available
- `db.get_contact_detail(uuid)` → `Option<ContactDetail>`
- `db.search_contacts(&words, case_sensitive, limit)` → `Vec<ContactListRow>`

### Output Format
See `/docs/features/cmd-show.md` for expected display format.

## References

- `/docs/features/cmd-show.md` - UX specification and output format
- `/docs/features/data-model.md` - ContactDetail structure
- `src/models/contact_detail.rs` - ContactDetail struct with helper methods

## Verification

```bash
cargo build
cargo test cli::show
cargo run -- show --help
cargo run -- show "John"  # Test name search
```
