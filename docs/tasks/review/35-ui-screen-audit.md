# Task 35: UI Screen Audit - Perfect Every Screen

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 28, 29, 30, 31, 32, 33
**Status**: Code complete, manual verification for user

## Problem

With Tasks 28-33 complete, we need to verify all changes work together consistently. This is the final audit before Task 34 (Testing and Polish).

## Current conventions (from completed tasks)

- **Navigation hints**: `[↑/↓]` vertical, `[←/→]` horizontal (arrow keys work, j/k/h/l also work)
- **Prompts**: lowercase with space: `search: `, `field [current]: `
- **Feedback**: single word: `Saved.`, `Deleted.`, `Cancelled.`
- **Truncation**: unicode ellipsis `…`
- **Empty states**: `No contacts.`, `No matches.`, `No messages.`
- **Selection**: inquire Select with `Name (email)` format
- **Confirmation**: inquire Confirm with default No

## Success criteria

### Main Menu (`menu.rs`)
- [x] Clean rendering, no artifacts
- [x] Vim keys work (j/k)
- [x] Escape exits cleanly
- [x] Prompts use `field: ` format (Task 29)
- [x] All options functional

### List View (`list.rs`)
- [x] Pagination displays correctly
- [x] Navigation: `1-20 of 100  [↑/↓] scroll [q]uit`
- [x] No decorative lines
- [x] Empty state: `No contacts.`
- [x] Review mode: `1/42  [e]dit [d]elete [←/→] [q]uit`
- [x] Raw mode only during key read (bug fix applied)
- [x] Uses ui::clear_screen()

### Contact Detail (`display.rs`, `show.rs`)
- [x] Name as plain header (no decoration)
- [x] Fields indented with 2 spaces
- [x] Only populated fields shown
- [x] Last message format: `< date "text…"` (Task 28)
- [x] Action bar: `[e]dit [m]essages [d]elete [q]uit`
- [x] Truncation uses `…` (Task 28)
- [x] Uses ui::clear_screen()

### Search (`search.rs`)
- [x] Results use review mode
- [x] No matches: `No matches.`
- [x] Review navigation: `[←/→]`
- [x] Uses ui::clear_screen()

### Selection Menus (`ui.rs` select_contact)
- [x] Use inquire Select for multiple matches (Task 29)
- [x] Format: `Name (email)`
- [x] Consistent across show/update/delete (Task 29)
- [x] Single match goes direct (no selection needed)

### Edit Forms (`list.rs`, `add.rs`)
- [x] Prompt: `field [current]: ` (Task 30)
- [x] Empty keeps current value
- [x] Validation: `Invalid email format` to stderr
- [x] Success: `Saved.` (Task 30)
- [x] Cancel: `Cancelled.` (Task 30)

### Delete (`delete.rs`)
- [x] Contact summary shown first (Task 31)
- [x] Use inquire Confirm
- [x] Prompt: `Delete Name? (y/N)` (Task 31)
- [x] Default No
- [x] Success: `Deleted.` (Task 31)
- [x] Review mode immediate: `Deleted: Name`

### Messages (`show.rs`)
- [x] Header: `Messages: Name`
- [x] Format: `> date "text…"` / `< date "text…"`
- [x] Navigation: `[↑/↓] scroll [q]uit` (updated)
- [x] Truncation uses `…` (Task 32)
- [x] Empty state: `No messages.`

### Error States (Task 33)
- [x] Errors to stderr (`eprintln!`, `anyhow::bail!`)
- [x] Exit code 1 on error (anyhow handles)
- [x] No stack traces to user (anyhow formats cleanly)
- [x] Ctrl+C: RawModeGuard RAII pattern

## User verification (optional)

Manual testing the user can do to verify:
1. Run through complete flow: menu → add → list → search → show → edit → delete → quit
2. Verify no visual glitches
3. Verify Ctrl+C works at each step
4. Test in narrow terminal (80 cols)

---
