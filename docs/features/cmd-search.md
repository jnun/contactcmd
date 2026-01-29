# Feature: search Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Find contacts matching a query across name, email, and other fields.

## User Stories

- As a user, I want to search by name so I can find someone quickly
- As a user, I want to search by email so I can find who owns an address
- As a user, I want multi-word search so I can narrow results

## Requirements

### Functional Requirements

- [ ] Search across display_name, search_name, email
- [ ] Multi-word queries use AND logic
- [ ] Case-insensitive by default
- [ ] Interactive selection for multiple results
- [ ] Direct display for single result

### Non-Functional Requirements

- [ ] Return results in <100ms
- [ ] Handle partial matches

## Technical Design

### CLI Interface

```
contactcmd search <QUERY> [OPTIONS]

Arguments:
  <QUERY>  Search terms

Options:
  -c, --case-sensitive  Case-sensitive matching
  -f, --field <FIELD>   Limit to field: name, email, phone, org
  -l, --limit <N>       Max results (default: 20)
```

### Search Logic

1. Split query into words
2. For each word, match against:
   - `search_name LIKE '%word%'`
   - `display_name LIKE '%word%'`
   - `email_address LIKE '%word%'`
3. AND all word conditions together

### Output: Single Result

Shows full contact details (same as `show` command).

### Output: Multiple Results

```
Found 3 contacts matching "john":

  1. John Smith (john@example.com) - Austin, TX
  2. John Doe (jdoe@work.com) - Denver, CO
  3. Johnny Appleseed - No email

Select [1-3], [a]ll, or [q]uit:
```

### Database Query

```sql
SELECT DISTINCT p.*
FROM persons p
LEFT JOIN emails e ON e.person_id = p.id
WHERE p.is_active = 1
  AND (
    p.search_name LIKE '%' || ?1 || '%'
    OR p.display_name LIKE '%' || ?1 || '%'
    OR e.email_address LIKE '%' || ?1 || '%'
  )
  AND (
    p.search_name LIKE '%' || ?2 || '%'
    OR p.display_name LIKE '%' || ?2 || '%'
    OR e.email_address LIKE '%' || ?2 || '%'
  )
ORDER BY p.sort_name
LIMIT ?
```

## Acceptance Criteria

- [ ] `search "john"` finds contacts with john in name
- [ ] `search "john smith"` finds contacts matching both words
- [ ] `search "@gmail"` finds gmail addresses
- [ ] Single result shows full details
- [ ] Multiple results show selection menu
- [ ] Case-insensitive by default
- [ ] Results in <100ms
