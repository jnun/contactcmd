# Task 32: UI Messages Screen

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - Messages Screen mockups, Message Direction

## Problem

The messages screen shows iMessage history for a contact. It's a read-only view for context.

Note: Much of this was addressed in Task 37 (Messages UI Consistency). This task focuses on the contact-specific messages view in `show.rs`.

## Success criteria

- [x] Header: `Messages: Contact Name` (done in Task 37)
- [x] Each message: `> date "text..."` or `< date "text..."` (done)
- [x] `>` for outgoing (sent), `<` for incoming (received) (done)
- [x] No borders or decorative elements (done)
- [x] Empty state: `No messages.` (done)
- [x] Verify scroll with j/k works in `show_messages_screen()`
- [x] Verify status line format: `1-15 of 47  [↑/↓] scroll [q]uit`
- [x] Newest messages first (verify ordering) - `ORDER BY m.date DESC`

## Notes

Target appearance:
```
Messages: John Smith

> Jan 28 "Hey, are we still on for lunch?"
< Jan 28 "Yes! See you at noon"
> Jan 27 "Thanks for sending the doc…"
< Jan 25 "Here's the proposal we dis…"
< Jan 20 "Great meeting you at the…"

1-15 of 47  [↑/↓] scroll [q]uit
```

Date format follows same rules as contact detail:
- Today: `12:30pm` (time only)
- This week: `Mon` (day only)
- This year: `Jan 28`
- Previous years: `Jan 28, 2025`

Consider: Should we support viewing full message text? Probably not in v1 - users can open Messages.app for full context.

---
