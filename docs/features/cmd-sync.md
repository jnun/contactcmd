# Feature: sync Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Synchronize contacts with macOS Contacts app.

## User Stories

- As a user, I want to import my existing contacts so I don't start from scratch
- As a user, I want to keep contacts in sync so changes propagate
- As a user, I want to preview changes before syncing

## Requirements

### Functional Requirements

- [ ] Request macOS Contacts permission
- [ ] Import all contacts from macOS
- [ ] Track external IDs to match contacts across syncs
- [ ] Update existing contacts on re-sync
- [ ] Preview mode (dry-run)
- [ ] Progress indicator for large imports

### Non-Functional Requirements

- [ ] Import 5,000 contacts in <5s
- [ ] Handle permission denial gracefully

## Technical Design

### CLI Interface

```
contactcmd sync mac [OPTIONS]

Options:
  -n, --dry-run   Preview changes without importing
  -f, --force     Overwrite local changes
  -v, --verbose   Show detailed progress
```

### Permission Flow

```
contactcmd sync mac

Requesting access to Contacts...
[System dialog appears]

Access granted. Starting sync...
```

If denied:
```
Access denied. Please grant permission in:
  System Settings > Privacy & Security > Contacts

Then run 'contactcmd sync mac' again.
```

### Import Progress

```
contactcmd sync mac

Syncing with macOS Contacts...

Fetching contacts... 6,095 found

Importing:
  [████████████████████████████████████████] 6095/6095

Results:
  Created:  5,892
  Updated:    198
  Skipped:      5 (no name)
  Errors:       0

Sync complete in 3.2s
```

### Dry Run

```
contactcmd sync mac --dry-run

Preview: Syncing with macOS Contacts...

Would import:
  New:      5,892 contacts
  Update:     198 contacts
  Skip:         5 contacts (no name)

No changes made.
```

### Field Mapping

| macOS Field | ContactCMD Field |
|-------------|------------------|
| identifier | external_ids.apple |
| givenName | name_given |
| familyName | name_family |
| middleName | name_middle |
| namePrefix | name_prefix |
| nameSuffix | name_suffix |
| nickname | name_nickname |
| emailAddresses | emails[] |
| phoneNumbers | phones[] |
| postalAddresses | addresses[] |
| organizationName | organization.name |
| jobTitle | person_organization.title |
| departmentName | person_organization.department |
| birthday | special_dates (type=birthday) |
| note | notes |

### Sync Logic

```
For each macOS contact:
  1. Extract identifier
  2. Search DB for external_ids.apple = identifier
  3. If found:
     - Compare fields
     - Update if different (macOS wins unless --force)
  4. If not found:
     - Create new person
     - Create related records (emails, phones, etc.)
     - Store external_ids.apple = identifier
```

### External ID Storage

```json
// person.external_ids column (JSON)
{
  "apple": "410FE041-5C4E-48DA-B4DE-04C15EA3DBAC"
}
```

## Acceptance Criteria

- [ ] Requests permission on first run
- [ ] Handles permission denial gracefully
- [ ] Imports all contacts from macOS
- [ ] Creates emails, phones, addresses
- [ ] Creates organizations
- [ ] Imports birthdays as special dates
- [ ] Tracks external IDs for re-sync
- [ ] Updates existing contacts on re-sync
- [ ] --dry-run previews without changes
- [ ] Shows progress for large imports
- [ ] Completes 5,000 contacts in <5s
