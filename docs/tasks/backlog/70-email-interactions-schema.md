# Task 70: Email Interactions Schema

## Summary

Create database schema to store email communication metadata.

## Dependency

None - this is the foundation.

## Blocked By

(none)

## Blocks

- Task 71: Spotlight email integration
- Task 72: Gmail metadata fetch
- Task 73: Outlook integration

## Scope

New table: `email_interactions`

| Field | Type | Notes |
|-------|------|-------|
| id | UUID | Primary key |
| person_id | UUID | Nullable - might not match known contact |
| email_address | TEXT | Address used in communication |
| subject | TEXT | Subject line |
| date | TEXT | ISO timestamp |
| direction | TEXT | 'sent' or 'received' |
| source | TEXT | 'gmail', 'macmail', 'outlook', 'spotlight' |
| message_id | TEXT | Unique, for deduplication |
| synced_at | TEXT | When we imported this |

Indexes: `person_id`, `date`, `email_address`, `message_id` (unique)

## Files

| File | Change |
|------|--------|
| `src/db/schema.rs` | Add migration |
| `src/db/mod.rs` | Add CRUD functions |

## Acceptance

- Migration runs successfully
- Can insert/query email interactions
- Unique constraint on message_id prevents duplicates
