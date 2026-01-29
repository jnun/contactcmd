# Task 30: UI Edit Forms

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - Form Archetype, [design-system.md](/docs/guides/design-system.md) - Inquire Patterns

## Problem

Edit forms appear in:
- Add new contact (all fields empty)
- Quick edit (name, email, phone, notes)
- Full edit (all fields including organization, addresses)
- Notes-only edit

Forms should be fast to complete, hard to make mistakes, and provide immediate feedback on errors.

UX principles:
- **Progressive disclosure**: Quick edit for common cases, full edit for power users
- **Inline validation**: Check email format before submission
- **Defaults reduce friction**: Empty = keep current value
- **Escape hatch**: Ctrl+C cancels without saving

## Success criteria

- [ ] Refactor `handle_edit()` and `handle_edit_all()` in list.rs
- [ ] Refactor `run_add()` in add.rs
- [ ] Use inquire Text for single-line fields
- [ ] Use inquire Editor for notes (multi-line)
- [ ] Prompt format: `field [current]: ` (shows current value in brackets)
- [ ] Empty input keeps current value
- [ ] Email validation with clear error: `Invalid email format`
- [ ] Success feedback: `Saved.` (single word)
- [ ] Cancel with Ctrl+C shows: `Cancelled.`
- [ ] No confirmation prompt for edit (save is the confirmation)
- [ ] After edit, return to detail view with updated data

## Notes

Quick edit flow:
```
first [John]:
last [Smith]: Smithson
email [john@example.com]:
phone [555-1234]: 555-9999
notes [Met at…]:

Saved.
```

Add flow (no current values):
```
first: John
last: Smith
email: john@example.com
phone:
notes:

Created: John Smith
```

Full edit adds:
- middle, nickname, prefix, suffix
- company, title, department
- All email addresses
- All phone numbers

Consider: Should quick edit become the default `[e]dit` and full edit require a separate command? Probably yes - most edits are quick fixes.

---
