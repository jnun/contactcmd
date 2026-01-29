# Task 11: Google Contacts Sync

**Feature:** /docs/features/cmd-sync.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement the `contactcmd sync google` command to import/export contacts with Google Contacts using the Google People API. This enables users to sync their contacts with Google's ecosystem.

## Success criteria

- [ ] OAuth2 authentication flow with Google
- [ ] Secure token storage in system keychain/credential store
- [ ] Handles auth failure gracefully with re-auth instructions
- [ ] Imports all contacts from Google Contacts
- [ ] Creates person records with proper name fields
- [ ] Creates email, phone, address records
- [ ] Creates organizations and person_organization links
- [ ] Imports birthdays as special_dates
- [ ] Tracks external_ids.google for re-sync matching
- [ ] Updates existing contacts on re-sync (by external ID)
- [ ] `--dry-run` previews changes without importing
- [ ] Progress indicator for large imports
- [ ] Completes 5,000 contacts in <10s

## Notes

Uses Google People API: https://developers.google.com/people

CLI interface:
```
contactcmd sync google [OPTIONS]
  --dry-run     Preview changes without importing
  --force       Overwrite local changes
  --verbose     Show detailed progress
  --logout      Clear stored credentials
```

Field mapping:
- resourceName → external_ids.google
- names[].givenName → name_given
- names[].familyName → name_family
- emailAddresses → emails[]
- phoneNumbers → phones[]
- addresses → addresses[]
- organizations[].name → organization.name
- organizations[].title → person_organization.title
- birthdays → special_dates (type=birthday)
