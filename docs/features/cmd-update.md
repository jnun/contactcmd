# Feature: update Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Modify an existing contact's information interactively or directly.

## User Stories

- As a user, I want to update contact info so I can keep data current
- As a user, I want interactive editing so I can see what I'm changing
- As a user, I want direct updates so I can script changes

## Requirements

### Functional Requirements

- [ ] Interactive mode with menu
- [ ] Direct mode with CLI options
- [ ] Update any field
- [ ] Add/remove emails, phones, addresses
- [ ] Add/remove tags
- [ ] Show what changed

### Non-Functional Requirements

- [ ] Update in <50ms

## Technical Design

### CLI Interface

```
contactcmd update <IDENTIFIER> [OPTIONS]

Arguments:
  <IDENTIFIER>  UUID or name

Options:
  -f, --first <NAME>       First name
  -l, --last <NAME>        Last name
  -e, --email <EMAIL>      Update primary email
  -p, --phone <PHONE>      Update primary phone
  -c, --company <NAME>     Company name
  -t, --title <TITLE>      Job title
  -n, --notes <TEXT>       Notes (replaces)
      --add-email <EMAIL>  Add email
      --add-phone <PHONE>  Add phone
      --add-tag <TAG>      Add tag
      --remove-tag <TAG>   Remove tag
```

### Interactive Mode

```
contactcmd update "John Smith"

Editing: John Smith (john@example.com)

What would you like to update?
  [1] Name
  [2] Email
  [3] Phone
  [4] Organization
  [5] Address
  [6] Tags
  [7] Notes
  [0] Save and exit

Choice: 2

Emails:
  1. john@example.com (personal) ★
  2. jsmith@work.com (work)

Options:
  [a] Add email
  [e] Edit email
  [d] Delete email
  [p] Set primary
  [b] Back

Choice: e
Which email (1-2): 1
New address: john.smith@gmail.com

Updated email.
```

### Direct Mode

```bash
# Update single field
contactcmd update "John Smith" --email new@email.com

# Update multiple fields
contactcmd update "John Smith" --title "Senior Engineer" --company "NewCorp"

# Add tag
contactcmd update "John Smith" --add-tag vip

# Output
Updated: John Smith
  Email: john@example.com → new@email.com
```

### Change Tracking

Track and display what changed:
```
Updated: John Smith
  Email: john@example.com → john.smith@gmail.com
  Title: Engineer → Senior Engineer
```

### Database Operations

1. Load person with all related records
2. Apply changes
3. Recompute display_name, sort_name, search_name if name changed
4. Set updated_at = now()
5. Set is_dirty = true (for sync)
6. Save in transaction

## Acceptance Criteria

- [ ] Interactive menu for editing
- [ ] Direct options for scripting
- [ ] Updates any field
- [ ] Adds/removes emails, phones
- [ ] Adds/removes tags
- [ ] Shows what changed
- [ ] Sets is_dirty for sync tracking
