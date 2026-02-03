# Task 64: Wire import command handler

**Feature**: none
**Created**: 2026-01-31
**Depends on**: Task 59, Task 60, Task 61, Task 62, Task 63
**Blocks**: Task 65

## Problem

This is the main import logic that ties everything together. When user runs `contactcmd import prospects.csv`, it should:

1. Parse CSV file
2. For each row:
   - Check if organization exists (by name) → skip if duplicate
   - Create organization
   - Create placeholder person (display_name = company name, person_type = 'business')
   - Link person → organization
   - Create phone record (if present)
   - Create email record (if present)
   - Create address record
3. Track counts (created, skipped, errors)

## Success criteria

- [ ] `contactcmd import prospects.csv` processes all rows
- [ ] Organizations created in database
- [ ] Persons created with person_type = 'business'
- [ ] Phones/emails/addresses attached to persons
- [ ] Duplicates detected and skipped
- [ ] `--dry-run` previews without inserting

## Notes

Test with small subset first. Full Alabama dataset has 4,923 rows.

Existing CRUD to use:
- Person insert: `src/db/persons.rs`
- Phone/email/address insert: `src/db/persons.rs`

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
