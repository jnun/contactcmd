# Task 74: Email Contact Matching

## Summary

Link email interactions to contacts by matching email addresses.

## Dependency

Requires at least one data source populated.

## Blocked By

- Task 70: Email interactions schema
- Task 71 OR Task 72 OR Task 73 (at least one source)

## Blocks

- Task 75: Email deduplication
- Task 76: Sync command

## Scope

Match `email_interactions.email_address` to `emails.email_address` to set `person_id`.

## Matching Strategy

| Scenario | Action |
|----------|--------|
| Exact match | Set person_id |
| Multiple contacts with same email | Pick primary or first match |
| No match | Leave person_id NULL |
| Create contact option | Prompt to create new contact |

## Files

| File | Change |
|------|--------|
| `src/db/mod.rs` | Add `match_email_to_contact()` function |
| `src/db/mod.rs` | Add `update_email_interaction_person()` function |

## Acceptance

- After sync, email_interactions have person_id populated where possible
- Can query "emails with unknown sender" (person_id IS NULL)
- Matching is case-insensitive
