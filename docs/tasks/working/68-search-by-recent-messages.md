# Task 68: Search Contacts by Recent Messages (SMS/iMessage)

**Feature**: none
**Created**: 2025-01-15
**Depends on**: none
**Blocks**: none

## Problem

User communicated with someone recently but forgot their name. Need to find contacts by message recency. This task covers iMessage/SMS only - email support is in Task 80.

## Scenario

I met someone and texted them in the last week or two. We talked about a job opportunity, and I need to nudge that contact to move the conversation forward.

But I don't remember their name or what day I texted them.

I need to find them so I can message them and set up a meeting.

**What I want:** See the people (just the people, not full message threads) from the last 2 weeks. Scan the list, recognize the name, then follow up.

## Success criteria

- [ ] `/recent` shows contacts messaged via iMessage/SMS in last 7 days
- [ ] `/recent 14` shows last 14 days, `/recent 30` shows last 30 days
- [ ] Output shows: name, time ago, channel (iMessage/SMS)
- [ ] List sorted by most recent first
- [ ] Works for contacts not yet viewed in app (live scan)

## Expected Output

```
Recent contacts (last 14 days):

  Sarah Chen         3 days ago    iMessage
  Alex Rivera        8 days ago    SMS
  Jordan Lee        12 days ago    iMessage

3 contacts. /browse to view details.
```

## Existing Infrastructure

| What | Where | Notes |
|------|-------|-------|
| `LastMessage` struct | `src/cli/messages/macos.rs:19` | Has `date: DateTime<Local>` |
| `get_last_message_for_handles()` | `src/cli/messages/macos.rs:239` | Queries iMessage DB directly |
| Special search syntax | `src/cli/search.rs:116-120` | Pattern: `email:missing` |
| Schema version | `src/db/schema.rs:1` | Currently V8 |

## Approach Options

### Option A: Live scan (no caching)
Query iMessage database directly for recent messages, match to contacts by phone/email.
- Pro: Always current, finds everyone
- Con: Slower, requires full scan each time

### Option B: Cache + sync
Store `last_message_at` in persons table, update via `/sync messages`.
- Pro: Fast queries after sync
- Con: Can miss contacts if not synced

### Option C: Hybrid
Cache for speed, but `/recent` does live scan if cache is stale.
- Pro: Best of both
- Con: More complex

## Files to Modify

| File | Change |
|------|--------|
| `src/db/schema.rs` | Add migration (if caching) |
| `src/db/persons.rs` | Add recency queries |
| `src/cli/messages/macos.rs` | Add bulk recent message scan |
| `src/cli/search.rs` | Handle `messaged:2weeks` syntax |
| `src/cli/chat.rs` | Add `/recent` command |

## New Commands

| Command | Description |
|---------|-------------|
| `/recent` | List contacts messaged in last 7 days |
| `/recent 14` | Last 14 days |
| `/recent 30` | Last 30 days |
| `/search messaged:2weeks` | Alternative syntax |

## Notes

- Email support will be added separately in Task 80 after email infrastructure (Tasks 70-76) is complete
- Start with live scan approach for simplicity; optimize with caching later if needed
