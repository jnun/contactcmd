# Task 13: vCard Sync

**Feature:** /docs/features/cmd-sync.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement vCard (.vcf) import and export commands. vCard is the standard format for contact exchange, supported by virtually all contact applications. This enables interoperability with any contact management system.

## Success criteria

- [ ] `contactcmd export vcard` exports all contacts to vCard format
- [ ] `contactcmd export vcard --output file.vcf` writes to specific file
- [ ] `contactcmd export vcard <id|name>` exports single contact
- [ ] `contactcmd import vcard file.vcf` imports contacts from vCard
- [ ] Supports vCard 3.0 format (RFC 2426)
- [ ] Supports vCard 4.0 format (RFC 6350)
- [ ] `--version 3|4` flag to specify export version
- [ ] Handles single-contact and multi-contact .vcf files
- [ ] Imports all standard vCard fields (N, FN, EMAIL, TEL, ADR, ORG, BDAY, NOTE)
- [ ] Tracks UID field for re-sync matching via external_ids.vcard
- [ ] `--dry-run` previews import changes
- [ ] `--update` updates existing contacts by UID match
- [ ] Progress indicator for large files
- [ ] Completes 5,000 contacts in <5s

## Notes

CLI interface:
```
contactcmd export vcard [OPTIONS] [ID|NAME]
  --output FILE     Output file (default: stdout)
  --version 3|4     vCard version (default: 4)
  --all             Export all contacts

contactcmd import vcard FILE [OPTIONS]
  --dry-run         Preview changes
  --update          Update existing by UID
  --skip-errors     Continue on parse errors
```

vCard field mapping:
- N → name_family;name_given;name_middle;name_prefix;name_suffix
- FN → computed from name fields
- EMAIL → emails[]
- TEL → phones[]
- ADR → addresses[]
- ORG → organization.name
- TITLE → person_organization.title
- BDAY → special_dates (type=birthday)
- UID → external_ids.vcard
- NOTE → notes

Consider using vcard crate for parsing.
