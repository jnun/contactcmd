# Task 71: Spotlight Email Integration

## Summary

Use macOS Spotlight to query email metadata across all local mail clients.

## Dependency

Requires Task 70 (schema) to store results.

## Blocked By

- Task 70: Email interactions schema

## Blocks

- Task 74: Contact matching
- Task 76: Sync command

## Why Spotlight

- Universal - indexes Mac Mail, Outlook, and other clients
- No auth required - it's local
- Already indexed - fast queries
- Covers users who don't use Gmail API

## Scope

Query Spotlight for email metadata:
- Sender/recipient
- Subject
- Date
- Message ID

## Access Methods

| Method | Notes |
|--------|-------|
| `mdfind` CLI | Simple, can call from Rust |
| Core Spotlight API | More control, requires Objective-C bridge |

## Query Examples

```bash
# Find emails from last 14 days
mdfind 'kMDItemContentType == "com.apple.mail.emlx" && kMDItemContentCreationDate > $time.today(-14)'

# Find emails with specific sender
mdfind 'kMDItemAuthors == "john@example.com"'
```

## Files

| File | Change |
|------|--------|
| `src/cli/email/spotlight.rs` | New - Spotlight query functions |
| `src/cli/email/mod.rs` | Export spotlight module |

## Acceptance

- Can query recent emails via Spotlight
- Extract: sender, recipient, subject, date, message_id
- Insert results into email_interactions table
