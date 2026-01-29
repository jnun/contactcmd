# Task 34: UI Testing and Final Polish

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 26, 27, 28, 29, 30, 31, 32, 33
**Design reference**: [design-system.md](/docs/guides/design-system.md) - complete guide; [ui-system.md](/docs/designs/ui-system.md) - all patterns

## Problem

After implementing individual screens, we need end-to-end testing to ensure the complete experience is consistent with the design system. Polish means fixing the small details that users notice subconsciously.

Testing dimensions:
- **Functional**: Every feature works as designed
- **Visual**: Consistent with design system (archetypes, spacing, feedback)
- **Performance**: Instant response (<100ms for UI actions)
- **Compatibility**: Works across terminal emulators
- **Antifragile**: Handles edge cases per Terminal Resilience patterns

## Context from completed tasks

**Navigation (updated in all files):**
- Vertical: `[↑/↓]` (arrow keys, j/k also work)
- Horizontal: `[←/→]` (arrow keys, h/l also work)
- Status lines: `1-20 of 100  [↑/↓] scroll [q]uit`
- Review mode: `1/42  [e]dit [d]elete [←/→] [q]uit`

**Forms (Task 30):**
- Prompt format: `field [current]: ` with current value in brackets
- Empty input keeps current value
- Success: `Saved.` / Cancel: `Cancelled.`
- Uses inquire Text with minimal_render_config()

**Selection (Task 29):**
- Uses inquire Select with vim mode
- Format: `Name (email)` for disambiguation
- Single match skips selection

**Delete (Task 31):**
- Shows contact summary, then `Delete Name? (y/N)`
- Success: `Deleted.`
- Review mode immediate delete: `Deleted: Name`

**Errors (Task 33):**
- Errors to stderr via `eprintln!` or `anyhow::bail!`
- Exit code 1 on error (anyhow handles this)
- Empty states: `No contacts.`, `No matches.`, `No messages.`

**Known fix applied:**
- List display raw mode bug fixed - only enable raw mode when reading keys

## Success criteria

**Code changes (complete):**
- [x] All navigation uses arrow symbols consistently (`[↑/↓]`, `[←/→]`)
- [x] All prompts follow `field [current]: ` format (Task 30)
- [x] All feedback follows design system (`Saved.`, `Deleted.`, etc.)
- [x] No visual glitches on screen clear/redraw (raw mode bug fixed)
- [x] Ctrl+C exits cleanly from any screen (RAII guards)
- [x] All existing unit tests pass (72 tests)

**Manual verification (for user):**
- [ ] Test on: macOS Terminal.app, iTerm2, VS Code terminal
- [ ] Test in: tmux, ssh session, narrow terminal (80 cols)
- [ ] Performance: Menu loads in <50ms, list loads in <100ms

**Deferred:**
- [ ] README updated with new UI screenshots/examples (separate documentation task)

## Notes

Test script outline:
```
1. Launch contactcmd (empty db) - expect "No contacts."
2. Add contact via menu - verify form prompts lowercase
3. List - verify columns, pagination, [↑/↓] scroll
4. Search - verify inquire Select, single match direct
5. Show - verify detail format, [e]dit [m]essages [d]elete [q]uit
6. Edit via [e] - verify field [current]: format, Saved.
7. Messages via [m] - verify scroll with [↑/↓]
8. Delete via [d] - verify immediate delete in review, Deleted: Name
9. Delete via menu - verify confirmation Delete Name? (y/N), Deleted.
10. Ctrl+C at various points - verify clean exit
11. Quit - verify clean exit
```

Polish checklist (all complete):
- [x] All navigation hints use arrow symbols `[↑/↓]` and `[←/→]`
- [x] All prompts lowercase: `search: `, `first [John]: `
- [x] Consistent spacing (1 blank line between sections, 2 spaces indent)
- [x] Truncation uses `…` (unicode ellipsis) everywhere
- [x] Date format: `Today at 3:42pm`, `Jan 15 at 11:00am`, `Jan 15, 2024 at 2:30pm`
- [x] Count format: `1-20 of 100`
- [x] No orphaned text after screen clear (clear_screen works correctly)
- [x] Cursor position correct after every operation

Performance targets:
- Menu render: <50ms
- List fetch + render (20 items): <100ms
- Search (full text): <200ms
- Detail load: <50ms

## Files to check

Key files for UI consistency:
- `src/cli/ui.rs` - shared primitives, minimal_render_config()
- `src/cli/list.rs` - list display, review mode, edit forms
- `src/cli/show.rs` - contact detail, messages screen
- `src/cli/search.rs` - search review mode
- `src/cli/delete.rs` - delete confirmation
- `src/cli/add.rs` - add form
- `src/cli/menu.rs` - main menu, prompt format
- `src/cli/display.rs` - print_full_contact(), date formatting

---
