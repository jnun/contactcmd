# Task 10: macOS Contacts Sync

**Feature:** /docs/features/cmd-sync.md
**Created:** 2026-01-27
**Completed:** 2026-01-28
**Depends on:** Task 3

## Problem

Implement the `contactcmd sync mac` command to import contacts from macOS Contacts app using Objective-C bindings.

## Success criteria

- [x] Requests CNContactStore authorization
- [x] Handles permission denial gracefully with instructions
- [x] Imports all contacts from macOS Contacts
- [x] Creates person records with proper name fields
- [x] Creates email, phone, address records
- [x] Creates organizations and person_organization links
- [x] Imports birthdays as special_dates
- [x] Tracks external_ids.apple for re-sync matching
- [x] Updates existing contacts on re-sync (by external ID)
- [x] `--dry-run` previews changes without importing
- [x] Progress indicator for large imports
- [x] Completes 6,094 contacts in ~3s

## Verification

```
$ cargo run -- sync mac --dry-run
Syncing contacts from macOS Contacts...
Found 6094 contacts

[DRY RUN] Would import the following contacts:

  [CREATE] Kevin Jessop
  [CREATE] Rod
  ...
Dry run complete.

$ cargo run -- sync mac
Syncing contacts from macOS Contacts...
Found 6094 contacts
Processing... 6000/6094
Sync complete: 6094 created, 0 updated, 0 skipped

$ cargo run -- sync mac   # Re-sync
Syncing contacts from macOS Contacts...
Found 6094 contacts
Processing... 6000/6094
Sync complete: 0 created, 6094 updated, 0 skipped

$ cargo run -- search "John Apel"
================================================================================
                                   JOHN APEL
================================================================================

BASICS
  Name:       John Apel
  Type:       Personal

CONTACT INFO
  Address:    Cullman, Alabama

WORK
  Title:      President & Owner
  Company:    Apel Machine & Supply Co., Inc.
  Status:     Current

================================================================================
```

## Implementation

- `src/cli/sync/mod.rs` - Sync dispatcher
- `src/cli/sync/macos.rs` - macOS-specific implementation using objc2-contacts
- `src/db/persons.rs` - Added insert_special_date, insert_organization, insert_person_organization, get_or_create_organization, find_person_by_external_id

## Notes

Uses objc2 and objc2-contacts crates for macOS integration with conditional compilation.

Field mapping:
- givenName → name_given
- familyName → name_family
- emailAddresses → emails[]
- phoneNumbers → phones[]
- postalAddresses → addresses[]
- organizationName → organization.name
- jobTitle → person_organization.title
- birthday → special_dates (with year_known flag)
