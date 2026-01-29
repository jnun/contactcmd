# Task 25: UI Design System - Consistent Visual Language

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Task 24

## Problem

A design system ensures every screen feels like part of the same application. Without it, screens drift into inconsistency. We need documented patterns that are easy to follow and hard to break.

Core principles (derived from decades of CLI/TUI research):
- **Minimal**: Show only what's needed, nothing more
- **Clean**: No decorative elements, whitespace is the design
- **Reliable**: Same patterns everywhere, no surprises
- **Simple**: One obvious way to do things
- **Fast**: Instant feedback, no unnecessary delays
- **Elegant**: Beauty through restraint
- **Antifragile**: Graceful degradation, works on any terminal
- **Intuitive**: Follows user expectations from decades of CLI conventions

## Success criteria

- [x] Create `docs/designs/ui-system.md` documenting all patterns
- [x] Define text hierarchy (headers, labels, values, hints)
- [x] Define spacing rules (when to use blank lines, indentation)
- [x] Define prompt formats (input, selection, confirmation)
- [x] Define feedback patterns (success, error, warning, info)
- [x] Define navigation conventions (consistent across all screens)
- [x] Define color usage (or explicit decision to avoid colors)
- [x] Create example mockups for each screen type in the design doc

## Notes

Patterns to document:

**Text Hierarchy**
```
Contact Name          <- header (the focus)
  detail value        <- indented content
  another value

prompt:               <- lowercase, colon, space
```

**Navigation**
```
[key] action          <- square brackets, lowercase
1-5 of 20             <- counts, no labels like "Showing"
```

**Feedback**
```
Created: John Smith   <- action: result
Error: invalid email  <- capital "Error:", specific message
```

Note on capitalization: Use capital "Error:" for consistency with `ui.rs`. Action words in feedback (Created, Deleted, Saved) are capitalized.

**Selection Menus**
```
Select:

  1. John Smith (john@example.com)
  2. Jane Smith (jane@example.com)

[1-2] or [q]:
```
- 2-space indent for numbered items
- Optional context in parentheses (email, org)
- Range prompt shows valid options

**Spacing**
- One blank line between logical sections
- No trailing decorations
- No leading banners

**Raw Mode Usage**
Use inquire for all prompts (Text, Select, Confirm). Raw mode (crossterm) is acceptable only for:
- Screen clearing (`clear_screen()`)
- Single-key immediate actions in interactive displays (e.g., `[e]dit [d]elete [q]uit` where waiting for Enter would feel sluggish)

When using raw mode for keys, always use RAII guard pattern to ensure cleanup on exit.

Reference: Apple Human Interface Guidelines (CLI section), GNU Coding Standards (user interfaces)

---
