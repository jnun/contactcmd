# Task 21: Extended Search Fields

**Feature:** none
**Created:** 2026-01-27
**Depends on:** Task 5

## Problem

The current search command only searches person names and email addresses. Users need to find contacts by other attributes such as notes content, city/location, and company/organization affiliation. For example, a user might want to find "everyone in Austin" or "everyone at Acme Corp" or "anyone I noted as 'met at conference'".

## Success criteria

- [x] Search matches against notes content (notes.content field)
- [x] Search matches against city names (addresses.city field)
- [x] Search matches against organization names (organizations.name via person_organizations join)
- [x] Multi-word AND logic applies across all searchable fields (e.g., "austin acme" finds people in Austin AND at Acme)
- [x] Case-insensitive search works for all new fields
- [x] Case-sensitive search (--case-sensitive flag) works for all new fields
- [ ] Performance: search with new fields completes in <100ms for 10,000 contacts
- [x] Existing name and email search behavior unchanged
- [x] Unit tests cover each new searchable field
- [x] Unit tests cover combined searches across multiple field types

## Notes

Current search implementation in `src/db/persons.rs:search_persons_multi()` searches:
- `persons.search_name` / `persons.display_name`
- `emails.email_address`

New fields to add:
- `notes.content` - free-text notes attached to contacts
- `addresses.city` - city field from addresses table
- `organizations.name` - company names via `person_organizations` join table

The query will need additional LEFT JOINs:
```sql
LEFT JOIN notes n ON n.person_id = p.id
LEFT JOIN addresses a ON a.person_id = p.id
LEFT JOIN person_organizations po ON po.person_id = p.id
LEFT JOIN organizations o ON o.id = po.organization_id
```

Consider adding indexes if performance testing shows they're needed:
- `idx_notes_content` on `notes(content)` - may not help with LIKE queries
- `idx_org_name` on `organizations(name)`
