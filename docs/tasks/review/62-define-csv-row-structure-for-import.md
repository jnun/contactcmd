# Task 62: Define CSV row structure for import

**Feature**: none
**Created**: 2026-01-31
**Depends on**: Task 58
**Blocks**: Task 64

## Problem

The CSV import needs a canonical format that contactcmd understands. For the first version, CSV headers must match exactly (case-insensitive). Later we'll add field mapping for non-matching CSVs.

Define the expected import format:

| Header | Required | Maps To |
|--------|----------|---------|
| `company_name` | Yes | organizations.name, persons.display_name |
| `street` | No | addresses.street |
| `city` | No | organizations.city, addresses.city |
| `state` | No | organizations.state, addresses.state |
| `zip_code` | No | addresses.postal_code |
| `phone` | No | phones.phone_number |
| `email` | No | emails.email_address |
| `website` | No | organizations.website |
| `industry` | No | organizations.industry |
| `external_id` | No | custom_metadata (for license numbers, etc.) |

## Success criteria

- [ ] `ImportRow` struct defined with canonical field names
- [ ] Serde `Deserialize` derived for CSV parsing
- [ ] Optional fields handled correctly (empty string → None)
- [ ] Unit test parsing a sample CSV row
- [ ] Clear error if required field `company_name` is missing

## Notes

**Location:** Add `ImportRow` struct to `src/cli/import.rs` (already exists with placeholder `run_import` function).

**Existing models to reference:**
- `Organization` in `src/models/organization.rs`: id, name, org_type, industry, website, city, state, country
- `PersonOrganization` in `src/models/organization.rs`: person_id, organization_id, relationship_type, is_current, is_primary

**Existing DB functions (from Tasks 60-61):**
- `db.search_organizations_by_name(name, city, state)` - dedup check
- `db.insert_organization(org)` - create org
- `db.insert_person_organization(po)` - link person to org

For Alabama contractors import, rename CSV headers to match:
- `company_name` ← was `name`
- `phone` ← was `phone_number`
- `external_id` ← was `license_number`

The field mapping feature (Task TBD) will handle mismatched headers automatically.

<!--
AI TASK CREATION GUIDE

Write as you'd explain to a colleague:
- Problem: describe what needs solving and why
- Success criteria: "User can [do what]" or "App shows [result]"
- Notes: dependencies, links, edge cases

Patterns that work well:
  Filename:    120-add-login-button.md (ID + kebab-case description)
  Title:       # Task 120: Add login button (matches filename ID)
  Feature:     **Feature**: /docs/features/auth.md (or "none" or "multiple")
  Created:     **Created**: 2026-01-28 (YYYY-MM-DD format)
  Depends on:  **Depends on**: Task 42 (or "none")
  Blocks:      **Blocks**: Task 101 (or "none")

Success criteria that verify easily:
  - [ ] User can reset password via email
  - [ ] Dashboard shows total for selected date range
  - [ ] Search returns results within 500ms

Get next ID: docs/STATE.md (5DAY_TASK_ID field + 1)
Full protocol: docs/5day/ai/task-creation.md
-->
