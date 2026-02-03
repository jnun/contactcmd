# Task 71: Gateway audit log CLI

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: none
**Blocks**: none

## Problem

Users have no way to review historical gateway activity. All message data exists in `communication_queue` but there's no CLI to view it. Users need to see what agents have sent, what was denied, and spot patterns of misuse.

## Success criteria

- [x] `contactcmd gateway history` shows past messages (approved, denied, sent, failed)
- [x] Output shows: timestamp, agent name, channel, recipient, status, subject/body preview
- [x] `--status` flag filters by status (e.g., `--status denied`)
- [x] `--agent` flag filters by API key name
- [x] `--limit N` controls how many entries to show (default 50)
- [x] Results ordered by most recent first

## Notes

- Reference: docs/ideas/ai-agent-gateway.md lines 141-149
- Consider adding `--since` and `--until` date filters in future
- Join with `api_keys` table to show agent name instead of raw key ID

## Implementation

- Added `History` subcommand to `GatewayCommands` in `src/cli/gateway/mod.rs`
- Added `list_queue_history()` function in `src/db/gateway.rs` with JOIN to get agent names
- Status filter uses exact match, agent filter uses LIKE for partial matching
- Failed entries show error message preview inline
