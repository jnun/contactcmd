# Task 27: UI List View Screen

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - List Archetype

## Problem

The list view shows contacts in a scannable format. Users need to quickly find contacts visually or navigate to pagination. Current implementation works but has inconsistent spacing and verbose prompts.

Key UX research findings:
- Users scan vertically, not horizontally (favor tall over wide)
- Alignment aids scanning (consistent column positions)
- Less is more (show primary info only, details on demand)

## Success criteria

- [ ] Refactor list display in `src/cli/list.rs`
- [ ] Clean column alignment: Name | Email/Phone | Location
- [ ] No decorative separators (no `───` lines)
- [ ] Pagination shows: `1-20 of 142`
- [ ] Navigation: `[↑/↓] scroll [q]uit` (vim-style per design system)
- [ ] Empty state: `No contacts.`
- [ ] Truncation uses `...` (per design system)
- [ ] Column widths adapt to terminal width (see Terminal Resilience in design system)
- [ ] Review mode follows Detail archetype

## Notes

Target appearance:
```
Name                          Email/Phone                  Location
John Smith                    john@example.com             Austin, TX
Jane Doe                      jane@company.org             New York, NY
Bob Wilson                    555-123-4567

1-20 of 142  [n]ext [p]rev [q]uit
```

Review mode:
```
John Smith

  john@example.com
  555-123-4567
  Austin, TX

1/142  [e]dit [d]elete [←/→] [q]uit
```

Single-line status, minimal chrome.

---
