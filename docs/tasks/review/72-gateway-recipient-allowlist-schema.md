# Task 72: Gateway recipient allowlist schema

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: none
**Blocks**: Task 73, Task 74

## Problem

Currently any API key can request messages to any recipient. A compromised key could message anyone in the contact database. Allowlists let admins restrict which recipients each agent can contact.

The database needs a table to store per-key allowlist entries.

## Success criteria

- [x] New table `api_key_allowlists` with columns: id, api_key_id, recipient_pattern, created_at
- [x] `recipient_pattern` supports exact match (email/phone) and wildcard (*@company.com)
- [x] Foreign key to `api_keys` table
- [x] Migration V11 creates table without affecting existing data

## Notes

- Reference: docs/ideas/ai-agent-gateway.md lines 98-103
- Pattern examples: `john@example.com`, `+15551234567`, `*@acme.com`
- Empty allowlist = unrestricted (don't break existing keys)

## Implementation

- Updated `SCHEMA_VERSION` to 11 in `src/db/schema.rs`
- Added `MIGRATION_V11` with CREATE TABLE for `api_key_allowlists`:
  - Foreign key to `api_keys` with ON DELETE CASCADE
  - Index on `api_key_id` for fast lookup
  - Unique index on `(api_key_id, recipient_pattern)` to prevent duplicates
- Added migration code in `src/db/mod.rs` for V10 → V11
- Added `AllowlistEntry` struct in `src/db/gateway.rs`
- Added database operations:
  - `insert_allowlist_entry()` - idempotent insert (returns false if duplicate)
  - `list_allowlist_entries()` - list all patterns for a key
  - `delete_allowlist_entry()` - remove by pattern
  - `has_allowlist()` - check if key has any restrictions (for enforcement)
- Added test `test_allowlist_crud` covering all operations
