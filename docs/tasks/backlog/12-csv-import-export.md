# Task 12: CSV Import/Export

**Feature:** /docs/features/cmd-sync.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement CSV import and export commands for contacts. This enables bulk data migration, spreadsheet editing, and integration with other tools that support CSV format.

## Success criteria

- [ ] `contactcmd export csv` exports all contacts to CSV
- [ ] `contactcmd export csv --output file.csv` writes to specific file
- [ ] `contactcmd import csv file.csv` imports contacts from CSV
- [ ] Auto-detects common CSV formats (Google, Apple, Outlook, LinkedIn)
- [ ] `--format` flag to specify source format explicitly
- [ ] `--mapping` flag to provide custom column mapping
- [ ] Handles multi-value fields (multiple emails/phones) via delimiter or multiple columns
- [ ] `--dry-run` previews import changes without applying
- [ ] `--update` flag to update existing contacts by email match
- [ ] Progress indicator for large files
- [ ] Validates CSV structure before import
- [ ] Reports import errors with line numbers
- [ ] Completes 10,000 contacts in <5s

## Notes

CLI interface:
```
contactcmd export csv [OPTIONS]
  --output FILE    Output file (default: stdout)
  --format FORMAT  Export format: default, google, apple, outlook

contactcmd import csv FILE [OPTIONS]
  --format FORMAT  Source format: auto, google, apple, outlook, linkedin
  --mapping FILE   Custom column mapping JSON
  --dry-run        Preview changes
  --update         Update existing contacts by email match
  --skip-errors    Continue on row errors
```

Default CSV columns:
- first_name, last_name, email, phone, company, title, address, birthday, notes

Multi-value handling options:
- Semicolon delimiter: "email1@x.com;email2@x.com"
- Numbered columns: email_1, email_2, email_3
