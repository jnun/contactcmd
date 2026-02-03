# Task 75: Email Deduplication

## Summary

Handle duplicate emails from multiple sources (e.g., same email from Spotlight and Gmail).

## Dependency

Requires multiple sources to have duplicates.

## Blocked By

- Task 74: Contact matching

## Blocks

- Task 76: Sync command (final integration)

## Problem

Same email might appear from:
- Spotlight (local Mac Mail)
- Gmail API (if also in Gmail)
- Outlook API (if forwarded)

## Dedup Strategy

| Field | Use |
|-------|-----|
| `message_id` | Primary dedup key (RFC 2822 Message-ID header) |
| `subject` + `date` + `from` | Fallback if message_id missing |

## Logic

1. On insert, check if message_id exists
2. If exists, update source field to note multiple sources
3. If not, insert new row

## Files

| File | Change |
|------|--------|
| `src/db/mod.rs` | Add `upsert_email_interaction()` with dedup logic |

## Acceptance

- Same email from multiple sources creates one row
- Source field tracks all sources (e.g., "spotlight,gmail")
- No duplicate emails in query results
