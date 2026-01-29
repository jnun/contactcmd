# Feature: delete Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Remove a contact with confirmation to prevent accidents.

## User Stories

- As a user, I want to delete contacts so I can remove outdated entries
- As a user, I want confirmation so I don't delete by accident
- As a user, I want to see details before confirming so I know what I'm deleting

## Requirements

### Functional Requirements

- [ ] Accept ID or name as identifier
- [ ] Show contact details before confirmation
- [ ] Require explicit confirmation
- [ ] Support force flag to skip confirmation
- [ ] Support batch deletion
- [ ] Cascade delete related records

### Non-Functional Requirements

- [ ] Delete in <50ms

## Technical Design

### CLI Interface

```
contactcmd delete <IDENTIFIER> [OPTIONS]

Arguments:
  <IDENTIFIER>  UUID or name

Options:
  -f, --force          Skip confirmation
      --batch <LIST>   Comma-separated names/IDs
```

### Confirmation Flow

```
contactcmd delete "John Smith"

Delete this contact?

  John Smith
  john@example.com
  Software Engineer at Acme Corp
  Austin, Texas

Type 'yes' to confirm: yes

Deleted: John Smith
```

### Batch Mode

```
contactcmd delete --batch "John Smith,Jane Doe"

Delete John Smith (john@example.com)?
Type 'yes' to confirm: yes
Deleted: John Smith

Delete Jane Doe (jane@example.com)?
Type 'yes' to confirm: no
Skipped: Jane Doe

Summary: 1 deleted, 1 skipped
```

### Force Mode

```bash
contactcmd delete "John Smith" --force

Deleted: John Smith
```

### Multiple Matches

```
contactcmd delete "John"

Found 3 contacts matching "John":
  1. John Smith (john@example.com)
  2. John Doe (jdoe@work.com)
  3. Johnny Appleseed

Select contact to delete [1-3] or [q]uit:
```

### Database Operations

1. Resolve identifier to person
2. Display confirmation
3. Delete with CASCADE (related records auto-delete)
4. Return success

### Cascade Deletes

Foreign key constraints handle cascading:
- emails (ON DELETE CASCADE)
- phones (ON DELETE CASCADE)
- addresses (ON DELETE CASCADE)
- person_organizations (ON DELETE CASCADE)
- person_tags (ON DELETE CASCADE)
- special_dates (ON DELETE CASCADE)
- interactions (ON DELETE CASCADE)
- notes (ON DELETE CASCADE)

## Acceptance Criteria

- [ ] `delete <uuid>` deletes by ID
- [ ] `delete "name"` searches and deletes
- [ ] Shows contact details before confirming
- [ ] Requires typing 'yes' to confirm
- [ ] --force skips confirmation
- [ ] --batch handles multiple contacts
- [ ] Cascades to related records
- [ ] Multiple matches show selection
