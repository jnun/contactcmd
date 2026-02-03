# Task 63: Add person_organizations link insert

**Feature**: none
**Created**: 2026-01-31
**Depends on**: Task 61
**Blocks**: Task 64

## Problem

After creating an organization and a placeholder person (for the company contact), we need to link them via the `person_organizations` junction table.

This establishes: "This person is the representative/contact for this organization."

## Success criteria

- [x] Function to insert person_organizations link
- [x] Sets relationship_type = 'representative'
- [x] Sets is_current = true, is_primary = true

## Notes

**STATUS: ALREADY IMPLEMENTED**

Function exists at `src/db/persons.rs:1250`:
```rust
pub fn insert_person_organization(&self, po: &PersonOrganization) -> Result<()>
```

Model at `src/models/organization.rs`:
```rust
impl PersonOrganization {
    pub fn new(person_id: Uuid, organization_id: Uuid) -> Self {
        // Sets relationship_type = "employee", is_current = true, is_primary = false
    }
}
```

**Note:** Default `relationship_type` is "employee". Task 64 should set it to "representative" when creating the link for imports.

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
