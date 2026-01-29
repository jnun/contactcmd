# Task 2: Database Schema and Migrations

**Feature:** /docs/features/data-model.md
**Created:** 2026-01-27
**Depends on:** Task 1
**Blocks:** Tasks 3-10

## User Story

As a user, I need a persistent SQLite database that stores my contacts and related data so that my information is saved between sessions.

## Acceptance Criteria

- [x] Database struct with open(), open_at(), and open_memory() methods
- [x] Auto-creates database file at platform config directory on first run
- [x] Schema version tracking for future migrations
- [x] All 11 tables created:
  - persons (core contact data)
  - emails, phones, addresses (contact methods)
  - organizations, person_organizations (work relationships)
  - tags, person_tags (categorization)
  - special_dates (birthdays, anniversaries)
  - notes (freeform text)
  - interactions (meeting/call history)
- [x] Foreign keys with ON DELETE CASCADE
- [x] Indexes on searchable/sortable columns
- [x] `cargo test db::` passes

## References

- /docs/features/data-model.md for entity relationships
