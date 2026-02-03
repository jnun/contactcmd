# Feature: recent Command

**Status:** IMPLEMENTED
**Created:** 2026-02-03
**Updated:** 2026-02-03

## Overview

Show contacts you've recently messaged via iMessage/SMS, with the ability to browse and take action on them.

## User Stories

- As a user, I want to see who I've texted recently so I can follow up
- As a user, I want to quickly access contacts I message frequently
- As a user, I want to see whether conversations were iMessage or SMS

## Requirements

### Functional Requirements

- [x] Query iMessage database for recent handles
- [x] Match handles to contacts in database
- [x] Show unknown numbers for contacts not in database
- [x] Default to 7 days, configurable via argument
- [x] Display service type (iMessage/SMS)
- [x] Store matched contacts in last_results for /browse

### Non-Functional Requirements

- [x] Graceful handling when Messages DB is inaccessible
- [x] Performance: handle large message histories efficiently

## Technical Design

### CLI Interface (Chat Mode)

```
/recent [days]
/r [days]

Arguments:
  [days]   Number of days to look back (default: 7)
```

### Output Format

```
Recent contacts (last 14 days):

  Sarah Chen         3 days ago    iMessage
  Alex Rivera        8 days ago    SMS
  +1 555-123-4567   12 days ago    iMessage  (unknown)

3 contacts, 1 unknown. /browse to view details.
```

### Database Query

```sql
SELECT
    h.id as handle,
    MAX(m.date) as last_date,
    (SELECT c.service_name
     FROM chat c
     INNER JOIN chat_handle_join chj ON c.ROWID = chj.chat_id
     WHERE chj.handle_id = h.ROWID
     ORDER BY c.last_read_message_timestamp DESC
     LIMIT 1) as service_name
FROM message m
INNER JOIN handle h ON m.handle_id = h.ROWID
WHERE m.date >= ?
GROUP BY h.id
ORDER BY last_date DESC
```

### Handle Matching

1. Load all persons from database
2. For each recent handle:
   - Check against all phones using `phones_match()` normalization
   - Check against all emails (case-insensitive)
   - If matched, add person to results
   - If not matched, display as "unknown"

## Acceptance Criteria

- [x] `/recent` shows contacts from last 7 days
- [x] `/recent 30` shows contacts from last 30 days
- [x] `/r` works as shortcut
- [x] Service type (iMessage/SMS) is displayed
- [x] Unknown numbers show with "(unknown)" marker
- [x] Matched contacts stored for `/browse`
- [x] Graceful error when Messages DB inaccessible
