# Task 26: UI Main Menu Screen

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25

## Problem

The main menu is the first screen users see. It sets expectations for the entire application.

The menu should:
- Load instantly
- Show options clearly
- Respond immediately to input
- Feel native to the terminal

## Success criteria

- [x] Rewrite `src/cli/menu.rs` using inquire Select
- [x] Menu appears immediately on `contactcmd` with no arguments
- [x] Options: List, Search, Show, Add, Note, Update, Delete, Sync, Messages, Quit
- [x] Vim keys work (j/k) for navigation, Enter to select
- [x] Screen clears cleanly on selection
- [x] Returns to menu after action completes (except Quit)
- [x] Ctrl+C and Escape exit gracefully
- [x] Works correctly in: standard terminal, VS Code, tmux, ssh
- [x] **Antifragile**: Command errors don't exit menu (catch, display, continue)
- [x] **Antifragile**: TTY check before entering interactive mode
- [x] **Antifragile**: Sync detects platform (not hardcoded "mac")

## Notes

Target appearance:
```
contactcmd

> List
  Search
  Show
  Add
  Note
  Update
  Delete
  Sync
  Messages
  Quit
```

Navigation follows inquire's standard model:
- `j/k` or arrow keys to move cursor
- `Enter` to select highlighted option
- `Esc` to quit
- Type to filter options

No header decorations, no footer instructions (inquire handles hints).
The `>` indicator shows current selection.

---
