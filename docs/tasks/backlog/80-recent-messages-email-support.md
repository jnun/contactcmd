# Task 80: Recent Messages - Email Support

**Feature**: none
**Created**: 2026-02-03
**Depends on**: Task 76 (email sync command)
**Blocks**: none

## Problem

Task 68 implements `/recent` for iMessage/SMS. This task extends that feature to include email contacts, so users can see everyone they've communicated with recently across all channels.

## Success criteria

- [ ] `/recent` includes contacts from email (Gmail, Mac Mail, Outlook)
- [ ] Email contacts show "Email" as the channel type
- [ ] Email and SMS/iMessage contacts merged and sorted by recency
- [ ] Deduplication when same contact appears in both email and SMS

## Expected Output

```
Recent contacts (last 14 days):

  Sarah Chen         3 days ago    iMessage
  Mike Johnson       5 days ago    Email
  Alex Rivera        8 days ago    SMS
  Jordan Lee        12 days ago    iMessage

4 contacts. /browse to view details.
```

## Dependencies

This task requires the email infrastructure from Task 69's sub-tasks:

| Task | Title | Required |
|------|-------|----------|
| 70 | Email interactions schema | Yes |
| 74 | Email contact matching | Yes |
| 75 | Email deduplication | Yes |
| 76 | Email sync command | Yes |

## Files to Modify

| File | Change |
|------|--------|
| `src/cli/chat.rs` | Extend `/recent` to query email_interactions |
| `src/db/persons.rs` | Add combined recency query (messages + email) |

## Notes

- Build on top of Task 68's implementation
- Reuse email_interactions table from Task 70
- Consider caching if combined query becomes slow
