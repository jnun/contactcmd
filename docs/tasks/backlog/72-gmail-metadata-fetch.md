# Task 72: Gmail Metadata Fetch

## Summary

Extend existing Gmail API integration to fetch email metadata (headers only, not body).

## Dependency

Requires Task 70 (schema) to store results.

## Blocked By

- Task 70: Email interactions schema

## Blocks

- Task 74: Contact matching
- Task 76: Sync command

## Existing Infrastructure

| What | Where |
|------|-------|
| Gmail API client | `src/cli/email.rs` |
| OAuth flow | `src/cli/google_auth.rs` |
| Token storage | Database settings table |

## Scope

Use Gmail API to fetch message list with headers:
- From/To addresses
- Subject
- Date
- Message ID

**Not fetching:** Body, attachments

## API Calls

| Endpoint | Purpose |
|----------|---------|
| `users.messages.list` | Get message IDs with date filter |
| `users.messages.get` | Get headers (format=metadata) |

## Files

| File | Change |
|------|--------|
| `src/cli/email.rs` | Add `fetch_recent_emails()` function |

## Acceptance

- Can fetch email metadata from Gmail API
- Respects date range filter (e.g., last 30 days)
- Extracts: from, to, subject, date, message_id
- Inserts into email_interactions table
- Uses existing OAuth tokens (no new auth flow)
