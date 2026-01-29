# Task 37: Messages UI Consistency

**Feature**: none
**Created**: 2026-01-28
**Completed**: 2026-01-28

## Problem

The messages viewing UI was inconsistent between different entry points. Fixed as part of Task 25 design system implementation.

## Success criteria

- [x] Remove decorative `---` lines from messages search
- [x] Consistent header format across both screens (`Messages: Name`)
- [x] Consistent navigation hints: `[←/→] contact [↑/↓] select [enter] expand [q]uit`
- [x] Consistent message format: `> date "text"` / `< date "text"`
- [x] Truncate for scan, full view on Enter (expand)
- [x] Consistent empty state: `No messages.`
- [x] Uses `ui::clear_screen()` everywhere
- [x] Removed ANSI colors (uses snippet centering instead for antifragility)

## Technical details

**Files to modify:**
- `src/cli/show.rs` - `show_messages_screen()` function
- `src/cli/messages/macos.rs` - search display functions

**Design decision needed:**
Should selecting a message show full text? Current UX:
- Contact messages: scroll through truncated list
- Search results: shows full message with highlighting

Proposed: Keep both but unify styling. Search can show full text since user is looking for specific content.

## Notes

Current inconsistencies:
| Aspect | show.rs | messages/macos.rs |
|--------|---------|-------------------|
| Header | `Messages: Name` | Varies |
| Decorations | None | `---` lines |
| Text | Truncated 50ch | Full wrapped |
| Navigation | `[↑/↓] scroll [q]uit` | `Press any key...` |
| Highlighting | None | ANSI yellow bold |

---
