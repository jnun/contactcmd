# Task 69: Gateway rate limiting schema

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: none
**Blocks**: Task 70

## Problem

AI agents can currently flood the gateway queue with unlimited message requests. Without rate limiting, a misbehaving or compromised agent could spam the queue, overwhelming human reviewers or exhausting send quotas (Gmail API limits, carrier SMS limits).

The database needs columns to track rate limit configuration and usage per API key.

## Success criteria

- [x] `api_keys` table has `rate_limit_per_hour` column (INTEGER, default 10)
- [x] `api_keys` table has `rate_limit_per_day` column (INTEGER, default 50)
- [x] `communication_queue` table can be queried for count of messages by api_key in time window
- [x] Migration V10 adds these columns without losing existing data

## Notes

- Reference: docs/ideas/ai-agent-gateway.md lines 89-96
- Default limits: 10/hour, 50/day per key
- Consider adding `cooldown_until` column for denial-triggered cooldowns (optional for MVP)

## Implementation

- Updated `SCHEMA_VERSION` to 10 in `src/db/schema.rs`
- Added `MIGRATION_V10` with ALTER TABLE statements for rate limit columns
- Added migration code in `src/db/mod.rs` for V9 → V10
- Updated `ApiKey` struct with `rate_limit_per_hour` and `rate_limit_per_day` fields
- Updated `find_api_key_by_hash()` and `list_api_keys()` queries to include new columns
- Added `count_queue_since()` function for rate limit checking
