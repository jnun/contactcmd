# Feature: show Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Display full details for a single contact.

## User Stories

- As a user, I want to see all info about a contact so I can reference it
- As a user, I want to look up by name so I don't need to know the ID

## Requirements

### Functional Requirements

- [ ] Accept ID or name as identifier
- [ ] Show all contact fields
- [ ] Display all emails, phones, addresses
- [ ] Show organization and job history
- [ ] Show tags and special dates
- [ ] Navigation when viewing from list/search

### Non-Functional Requirements

- [ ] Display in <50ms

## Technical Design

### CLI Interface

```
contactcmd show <IDENTIFIER>

Arguments:
  <IDENTIFIER>  UUID or name to search
```

### Output Format

```
================================================================================
                              JOHN SMITH
================================================================================

BASICS
  Name:       John Smith
  Nickname:   Johnny
  Type:       Business

CONTACT INFO
  Email:      john@example.com (personal) ★
              jsmith@acme.com (work)
  Phone:      (555) 123-4567 (mobile) ★
              (555) 987-6543 (work)
  Address:    123 Main St
              Austin, TX 78701

WORK
  Title:      Software Engineer
  Company:    Acme Corp
  Department: Engineering
  Since:      January 2020

TAGS
  colleague, tech, austin

DATES
  Birthday:   March 15, 1985

NOTES
  Met at tech conference 2024. Interested in AI projects.

================================================================================
ID: 550e8400-e29b-41d4-a716-446655440000
Created: 2024-01-15 | Updated: 2024-06-20
================================================================================
```

### Navigation Keys (when browsing)

| Key | Action |
|-----|--------|
| `n`, `→` | Next contact |
| `p`, `←` | Previous contact |
| `e` | Edit this contact |
| `d` | Delete this contact |
| `q` | Back/quit |

### Identifier Resolution

1. Try parsing as UUID
2. If not UUID, search by name
3. If multiple matches, show selection menu
4. If single match, display

## Acceptance Criteria

- [ ] `show <uuid>` displays contact by ID
- [ ] `show "John Smith"` searches and displays
- [ ] Shows all emails, phones, addresses
- [ ] Shows organization info
- [ ] Shows tags and special dates
- [ ] Shows notes
- [ ] Multiple name matches show selection
- [ ] Navigation works when browsing
