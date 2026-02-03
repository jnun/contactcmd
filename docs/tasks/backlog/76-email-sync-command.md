# Task 76: Email Sync Command

## Summary

Add `/sync email` command to trigger email metadata sync.

## Dependency

Requires integrations and dedup logic.

## Blocked By

- Task 74: Contact matching
- Task 75: Email deduplication
- At least one of: Task 71, 72, or 73

## Blocks

- Task 68: Recent messages integration

## Scope

New command to sync email metadata from all configured sources.

## Command

```
/sync email           # Sync last 30 days (default)
/sync email 14        # Sync last 14 days
/sync email 90        # Sync last 90 days
```

## Flow

1. Check which sources are configured (Spotlight always, Gmail if authed, Outlook if authed)
2. Fetch from each source with date filter
3. Deduplicate
4. Match to contacts
5. Report stats

## Output

```
Syncing email (last 30 days)...

  Spotlight:  142 emails
  Gmail:       89 emails
  Duplicates:  34 removed

Matched 156 emails to 43 contacts.
41 emails from unknown senders.

Done.
```

## Files

| File | Change |
|------|--------|
| `src/cli/chat.rs` | Add Sync variant handling for "email" |
| `src/cli/sync/email.rs` | New - orchestration logic |

## Acceptance

- `/sync email` fetches from all available sources
- Respects date range
- Deduplicates across sources
- Matches to contacts
- Reports summary stats
