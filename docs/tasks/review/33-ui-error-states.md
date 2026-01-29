# Task 33: UI Error Handling and Edge Cases

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25, 26, 27, 28, 29, 30, 31, 32
**Design reference**: [design-system.md](/docs/guides/design-system.md) - Error Recovery, CLI Conventions; [ui-system.md](/docs/designs/ui-system.md) - Feedback Patterns, Empty States, Terminal Resilience

## Problem

Errors and edge cases reveal the true quality of a UI. Poorly handled errors destroy user trust. Every error state needs a designed response.

Categories:
1. **User errors**: Invalid input, not found, validation failures
2. **System errors**: Database issues, permissions, disk full
3. **Edge cases**: Empty database, terminal resize, interrupted operations

Principle: **Errors should be helpful, not scary**. Tell users what happened and what to do next.

## Success criteria

- [x] Audit all error paths in CLI modules
- [x] Errors go to stderr, output to stdout (per CLI Conventions)
- [x] Exit code 1 on error, 0 on success (anyhow handles this)
- [x] User errors: `Error: specific message` (anyhow format)
- [x] Empty states per design system:
  - [x] List: `No contacts.`
  - [x] Search: `No matches.`
  - [x] Messages: `No messages.`
- [x] Graceful degradation per design system hierarchy
- [x] Permission errors: Clear message about what permission is needed
- [x] Ctrl+C: Clean exit via RAII guards (RawModeGuard pattern)
- [x] Terminal resize: Handled per Terminal Resilience patterns (clear_screen redraws)

## Notes

Error message format:
```
Error: could not open database
Error: permission denied accessing ~/Library/Messages
Invalid email format
No contacts found matching "xyz"
```

Lowercase "error" when inline, capitalized when it's the focus.
No stack traces to users. Log details internally.

Edge case behaviors:
- Empty database + List = `No contacts.` (no "add your first" tips)
- Search with no query = show all (same as list)
- Delete last contact = return to menu (list would be empty)
- Sync with no new contacts = `No new contacts.`

Testing: Create test cases for each error path. Errors are part of the UI.

---
