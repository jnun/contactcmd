# Task 29: UI Search and Selection Screens

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - Menu Archetype, Selection Menu format

## Problem

Search and selection appear in multiple contexts:
- Main search command
- Show command with ambiguous name
- Update command with ambiguous name
- Delete command with ambiguous name

All should feel identical. Currently they have slight variations in prompts and formatting.

Key UX principle: **Recognition over recall** - show matches immediately, let users select rather than re-type.

## Success criteria

- [ ] Unify selection UI across show, update, delete commands
- [ ] Search prompt: `search: ` (lowercase)
- [ ] Results use inquire Select for choosing among matches
- [ ] Single match goes directly to detail (no selection needed)
- [ ] No match shows: `No matches.`
- [ ] Selection shows: name + email (parenthetical) for disambiguation
- [ ] Review mode after search uses same navigation as list review
- [ ] Search is case-insensitive by default
- [ ] Empty search in menu context shows all contacts (becomes list)

## Notes

Selection appearance:
```
> John Smith (john@example.com)
  John Smith (jsmith@other.org)
  Johnny Smith (johnny@gmail.com)
```

Using inquire Select handles:
- Arrow key navigation
- Type-ahead filtering
- Vim keys (with config)
- Consistent rendering

For search results with review mode:
```
John Smith

  john@example.com
  Austin, TX

1/3  [e]dit [d]elete [←/→] [q]uit
```

Same pattern as list review, just with filtered results.

---
