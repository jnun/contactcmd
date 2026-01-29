# Task 24: UI Foundation - Select and Integrate TUI Library

**Feature**: none
**Created**: 2026-01-28
**Blocks**: Tasks 25, 26, 27, 28, 29, 30, 31, 32

## Problem

The current UI uses raw crossterm which causes issues (newlines in raw mode, inconsistent rendering) and requires significant boilerplate for each screen. We need a solid foundation that handles terminal quirks automatically and provides consistent primitives.

After researching options:
- **inquire**: Modern, beautiful defaults, handles edge cases, active maintenance
- **dialoguer**: Lightweight but less polished
- **ratatui**: Full TUI framework, overkill for our needs

Recommendation: **inquire** - it provides Select, MultiSelect, Text, Confirm, and Editor prompts with proper terminal handling out of the box.

## Success criteria

- [x] Add `inquire` to Cargo.toml dependencies
- [x] Remove raw crossterm usage from menu.rs (keep crossterm for screen clearing only)
- [x] Create `src/cli/ui.rs` module with shared UI primitives
- [x] Document UI conventions in code comments
- [x] Verify terminal rendering works correctly on: macOS Terminal, iTerm2, VS Code terminal
- [x] All existing tests still pass

## Notes

Key inquire features we'll use:
- `Select` for menus and single selection
- `Text` for input fields
- `Confirm` for yes/no prompts
- `Editor` for multi-line text (notes)

Design principles to establish:
- No decorative borders or lines
- Consistent prompt format: `label: ` (lowercase, colon, space)
- Navigation hints in brackets: `[↑/↓]` for vertical, `[←/→]` for horizontal
- Minimal vertical spacing

---
