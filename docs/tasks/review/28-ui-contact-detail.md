# Task 28: UI Contact Detail Screen

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - Detail Archetype

## Problem

The contact detail screen shows a single contact's full information. It's the "read" view before editing. Current implementation removed the heavy borders but still has formatting inconsistencies.

Design considerations:
- Name is the anchor, most prominent element
- Information hierarchy: identity → contact methods → location → metadata
- Only show fields that have values (no empty labels)
- Last message preview adds context (when available)

## Success criteria

- [ ] Refactor `src/cli/display.rs` `print_full_contact()`
- [ ] Name as header (plain text, not decorated)
- [ ] Organization/title on one line: `Title at Company`
- [ ] Contact info indented with 2 spaces
- [ ] No field labels for obvious data (email looks like email)
- [ ] Notes truncated with `…` if long (full notes in edit mode)
- [ ] Last message: `> date "preview…"` or `< date "preview…"`
- [ ] Action bar: `[e]dit [m]essages [d]elete [q]uit`
- [ ] No `=` or `-` decorative lines
- [ ] Consistent with list view patterns

## Notes

Target appearance:
```
John Smith

  CEO at Acme Corp
  john@example.com
  john.personal@gmail.com
  555-123-4567
  Austin, TX
  Met at conference, interested in…

  < Jan 15 at 3:42pm "Thanks for the intro…"

[e]dit [m]essages [d]elete [q]uit: _
```

The `<` indicates incoming message, `>` outgoing.
Date format per ui-system.md: `Today/Yesterday at H:MMam/pm`, `Mon D at H:MMam/pm` for this year, `Mon D, YYYY at H:MMam/pm` for previous years.

Fields appear in consistent order:
1. Organization/title
2. Emails (primary first)
3. Phones (primary first)
4. Location (city, state)
5. Notes (truncated)
6. Last message

---
