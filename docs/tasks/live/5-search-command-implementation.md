# Task 5: Search Command Implementation

**Feature:** /docs/features/cmd-search.md
**Created:** 2026-01-27
**Depends on:** Task 3, Task 4

## User Story

As a user, I want to search my contacts by name or email so that I can quickly find specific people without scrolling through the full list.

## Acceptance Criteria

- [x] `search "john"` finds contacts with john in name
- [x] `search "john smith"` uses AND logic (both words must match)
- [x] `search "@gmail"` searches email addresses
- [x] Case-insensitive by default
- [x] `--case-sensitive` flag for exact matching
- [x] `--limit N` limits results (default 20)
- [x] Single result shows full details automatically
- [x] Multiple results show numbered selection menu
- [x] Selection menu supports: number selection, [a]ll, [q]uit
- [x] Batch fetches display info (no N+1 queries for result list)
- [x] `cargo test search` passes (11 tests)

## References

- /docs/features/cmd-search.md for UX details
- src/cli/search.rs for implementation
