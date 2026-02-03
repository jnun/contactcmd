# Task 69: Email Communication Tracking (Meta)

## Sub-Tasks

| Task | Title | Depends On |
|------|-------|------------|
| 70 | Email interactions schema | - |
| 71 | Spotlight email integration | 70 |
| 72 | Gmail metadata fetch | 70 |
| 73 | Outlook integration | 70 |
| 74 | Email contact matching | 70, (71 or 72 or 73) |
| 75 | Email deduplication | 74 |
| 76 | Email sync command | 74, 75 |
| 80 | Recent messages (email part) | 76 |

## Dependency Graph

```
        ┌─────────────────────────────────────┐
        │      Task 70: Schema (foundation)   │
        └─────────────────┬───────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
   Task 71:          Task 72:          Task 73:
   Spotlight         Gmail API         Outlook API
        │                 │                 │
        └─────────────────┼─────────────────┘
                          ▼
              Task 74: Contact Matching
                          │
                          ▼
              Task 75: Deduplication
                          │
                          ▼
              Task 76: Sync Command
                          │
                          ▼
              Task 80: /recent integration
```

---

## Problem

Mac users use multiple email clients (Mac Mail, Outlook, Gmail via browser). Need to track email communication metadata with contacts without syncing entire email history.

## Goal

Know who I emailed, when, and about what - so I can:
- Find "people I emailed recently" (ties to Task 68)
- See communication history per contact
- Search by subject line later

## Scope

**In scope:**
- Email metadata (who, when, subject, direction)
- Multiple client support (Mac Mail, Outlook, Gmail)
- Link emails to contacts via email address matching
- Incremental sync (not full history)

**Out of scope (for now):**
- Full email body search
- Attachment tracking
- Email sending (already exists via Gmail API)

## Data Model

New table: `email_interactions`

| Field | Type | Notes |
|-------|------|-------|
| id | UUID | Primary key |
| person_id | UUID | Nullable - might not match known contact |
| email_address | TEXT | Address used in communication |
| subject | TEXT | Subject line |
| date | TEXT | ISO timestamp |
| direction | TEXT | 'sent' or 'received' |
| source | TEXT | 'gmail', 'macmail', 'outlook' |
| message_id | TEXT | Unique, for deduplication |
| synced_at | TEXT | When we imported this |

Index on: `person_id`, `date`, `email_address`

## Integration Points

| Source | Access Method | Auth | Notes |
|--------|---------------|------|-------|
| Gmail | Gmail API | OAuth (existing) | Already have sending integration |
| Mac Mail | Spotlight or ~/Library/Mail/ | None (local) | Universal for local clients |
| Outlook | Microsoft Graph API | OAuth (new) | Separate integration |
| Spotlight | mdfind / Core Spotlight | None (local) | Indexes all email clients |

## Approach Options

### Option A: Spotlight First
Use macOS Spotlight to query all email metadata universally.
- Works across all clients that integrate with Spotlight
- No additional auth needed
- Limited to what Spotlight indexes

### Option B: API Per Client
Build separate integrations for Gmail, Outlook, Mac Mail.
- More control and completeness
- Multiple auth flows
- More maintenance

### Option C: Hybrid
Spotlight for discovery, APIs for deeper sync.

## Sync Strategies

| Strategy | Description |
|----------|-------------|
| Manual | User runs `/sync email` |
| On-demand | Sync when viewing a contact |
| Background | Periodic sync (requires daemon) |
| Date-limited | Only last N days |

## Dependencies

- Task 68 (recent messages) - shares UI for "recent contacts"
- Gmail API integration (existing in `src/cli/email.rs`)
- OAuth flow (existing in `src/cli/google_auth.rs`)

## Sub-tasks (future)

1. [ ] Design email_interactions schema
2. [ ] Spotlight integration for Mac Mail
3. [ ] Gmail API metadata fetch (headers only)
4. [ ] Microsoft Graph API for Outlook (new OAuth)
5. [ ] Contact matching by email address
6. [ ] Deduplication logic (same email, multiple sources)
7. [ ] `/sync email` command
8. [ ] Integrate with `/recent` command (Task 80)
9. [ ] Per-contact email history view

## Open Questions

1. Spotlight vs APIs - which first?
2. How far back to sync? 30 days? 90 days? User configurable?
3. How to handle emails not matching any contact?
4. Privacy controls - let user exclude certain accounts?

## Related

- Task 80: Recent messages - email support (consumer of this data)
- Task 68: Recent messages - SMS/iMessage (independent, no email dependency)
- `src/cli/email.rs`: Existing Gmail sending
- `src/cli/google_auth.rs`: Existing OAuth flow
