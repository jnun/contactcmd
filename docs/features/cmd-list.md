# Feature: list Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Display contacts with pagination, sorting, and interactive navigation.

## User Stories

- As a user, I want to see my contacts in a list so I can browse them
- As a user, I want to page through results so I'm not overwhelmed
- As a user, I want to sort contacts so I can find who I'm looking for

## Requirements

### Functional Requirements

- [ ] Display contacts with name, email, phone, location
- [ ] Paginate results (default 20 per page)
- [ ] Sort by name, created date, updated date
- [ ] Navigate with keyboard in interactive mode
- [ ] Show total count and current position

### Non-Functional Requirements

- [ ] Display first page in <50ms
- [ ] Handle 10,000+ contacts smoothly

## Technical Design

### CLI Interface

```
contactcmd list [OPTIONS]

Options:
  -p, --page <N>      Page number (default: 1)
  -l, --limit <N>     Items per page (default: 20)
  -s, --sort <FIELD>  Sort by: name, created, updated
  -o, --order <DIR>   Sort direction: asc, desc
  -a, --all           Show all (no pagination)
```

### Output Format

```
Contacts (Page 1 of 50)
================================================================================
John Smith
  Software Engineer at Acme Corp
  john@example.com | (555) 123-4567
  Austin, Texas

Jane Doe
  Designer
  jane@example.com
  San Francisco, California

--------------------------------------------------------------------------------
[N]ext  [P]rev  [Q]uit                                    Showing 1-20 of 1000
```

### Interactive Keys

| Key | Action |
|-----|--------|
| `n`, `→`, `Space` | Next page |
| `p`, `←`, `b` | Previous page |
| `q`, `Esc` | Quit |
| `1-9` | Jump to page |

### Database Query

```sql
SELECT p.*,
       e.email_address as primary_email,
       ph.phone_number as primary_phone,
       a.city, a.state
FROM persons p
LEFT JOIN emails e ON e.person_id = p.id AND e.is_primary = 1
LEFT JOIN phones ph ON ph.person_id = p.id AND ph.is_primary = 1
LEFT JOIN addresses a ON a.person_id = p.id AND a.is_primary = 1
WHERE p.is_active = 1
ORDER BY p.sort_name ASC
LIMIT ? OFFSET ?
```

## Acceptance Criteria

- [ ] Shows contacts with name, work, email, phone, location
- [ ] Paginates with --page and --limit
- [ ] Sorts with --sort and --order
- [ ] Interactive navigation works (n/p/q keys)
- [ ] Shows "No contacts found" when empty
- [ ] Displays in <50ms for first page
