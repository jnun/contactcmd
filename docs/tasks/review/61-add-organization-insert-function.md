# Task 61: Add organization insert function

**Feature**: none
**Created**: 2026-01-31
**Depends on**: none
**Blocks**: Task 63, Task 64

## Problem

The import needs to create new organization records. We need an insert function for the organizations table.

Fields to populate:
- local_id (UUID)
- name (company name)
- city, state
- website (if available)
- industry (parsed from specialty)
- org_type ('contractor')
- custom_metadata (JSON with license_number, import_source)

## Success criteria

- [ ] Function to insert new organization with all fields
- [ ] Returns the created organization's local_id
- [ ] Handles custom_metadata as JSON

## Notes

Check if organization insert already exists in `src/db/`. The macOS sync might create organizations.

Schema reference:
```sql
organizations(local_id, name, legal_name, org_type, industry, city, state,
              country, website, linkedin_url, employee_count, parent_org_id,
              custom_metadata, created_at, updated_at, is_dirty, sync_status)
```

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
