# Task 76: Gateway content filter enforcement

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Depends on**: Task 75
**Blocks**: none

## Problem

Once content filters exist in the database, the gateway server must check message bodies against them. Messages matching 'deny' patterns should be rejected immediately. Messages matching 'flag' patterns should be queued but marked for extra attention.

## Success criteria

- [x] POST `/gateway/send` returns HTTP 400 if body matches a 'deny' filter
- [x] Error response includes which filter triggered and why
- [x] 'flag' filter matches queue message with `flagged` status indicator
- [x] Regex patterns are compiled once and cached for performance
- [x] Disabled filters are skipped
- [x] Subject line is also checked for email messages

## Implementation

**New module** (`src/cli/gateway/filter.rs`):
- `ContentFilterMatcher` struct with cached compiled patterns
- `FilterResult` enum: `Passed`, `Denied { filter_name, description }`, `Flagged { ... }`
- `check_email()` checks subject then body
- `check_message()` checks body only (SMS/iMessage)
- `reload()` loads enabled filters from DB and compiles regex patterns once
- 7 comprehensive tests

**Types** (`src/cli/gateway/types.rs`):
- Added `Flagged` variant to `QueueStatus` enum
- Added `ContentBlockedErrorResponse` struct for HTTP 400 responses

**Server** (`src/cli/gateway/server.rs`):
- `ContentFilterMatcher` stored in `GatewayServer` struct
- Filters loaded once at server startup
- Content check happens after rate limiting, before queue insertion
- Deny match → HTTP 400 with `ContentBlockedErrorResponse`
- Flag match → Message queued with "flagged" status

**Database** (`src/db/gateway.rs`):
- `list_pending_queue()` now includes flagged entries, sorted first
- `count_pending_queue()` includes flagged entries

**Approval UI** (`src/cli/gateway/approve.rs`):
- Flagged entries marked with `!` indicator in list view
- Detail view shows "FLAGGED - Review carefully" warning

**ReDoS protection**: The `regex` crate is designed to avoid catastrophic backtracking by using finite automata.

## Notes

- Check content after authentication and rate limiting, before queuing
- Error format: `{"error": "content_blocked", "filter": "SSN pattern", "description": "..."}`
