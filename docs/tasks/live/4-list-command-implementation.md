# Task 4: List Command Implementation

**Feature:** /docs/features/cmd-list.md
**Created:** 2026-01-27
**Depends on:** Task 3

## User Story

As a user, I want to list my contacts with pagination and sorting so that I can browse through my contact database efficiently.

## Acceptance Criteria

- [x] `contactcmd list` shows contacts with name, email, phone, location
- [x] `--page N` jumps to page N
- [x] `--limit N` sets items per page (default 20)
- [x] `--sort name|created|updated` sorts results
- [x] `--order asc|desc` sets sort direction
- [x] `--all` shows all contacts without pagination
- [x] Interactive navigation: n/p/q keys for next/prev/quit
- [x] Shows "No contacts found" when database is empty
- [x] Single-query performance (no N+1 queries)
- [x] `cargo test cli::list` passes (5 tests)

## References

- /docs/features/cmd-list.md for UX details
