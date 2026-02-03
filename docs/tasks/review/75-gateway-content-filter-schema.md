# Task 75: Gateway content filter schema

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Depends on**: none
**Blocks**: Task 76

## Problem

AI agents might accidentally include sensitive data in message bodies (SSNs, credit card numbers, passwords). Content filters can auto-deny messages matching dangerous patterns, providing a safety net beyond human review.

The database needs a table to store filter patterns.

## Success criteria

- [x] New table `content_filters` with columns: id, pattern, pattern_type, action, description, enabled, created_at
- [x] `pattern_type` supports: 'regex', 'literal'
- [x] `action` supports: 'deny', 'flag' (flag = allow but highlight for review)
- [x] `enabled` boolean allows disabling filters without deleting
- [x] Migration V12 creates table with sensible default filters (V10/V11 already used)

## Implementation

**Schema** (`src/db/schema.rs`):
- MIGRATION_V12 creates `content_filters` table with CHECK constraints
- Indexes on `enabled` and `action` for efficient enforcement queries

**Database operations** (`src/db/gateway.rs`):
- `ContentFilter` struct
- `insert_content_filter()`, `list_content_filters()`, `list_enabled_content_filters()`
- `get_content_filter()`, `set_content_filter_enabled()`, `delete_content_filter()`
- `seed_content_filters()` for default filters

**Default filters** (seeded on migration):
1. SSN pattern `\b\d{3}-\d{2}-\d{4}\b` (regex, deny)
2. Credit card `\b(?:\d{4}[- ]?){3}\d{4}\b` (regex, deny)
3. "password" (literal, flag)
4. API key/secret patterns (regex, deny)

**Tests**: 4 new tests covering seeding, CRUD, enabled filtering, and regex validation

## Notes

- Reference: docs/ideas/ai-agent-gateway.md lines 105-113
- Consider global filters (all keys) vs per-key filters in future
