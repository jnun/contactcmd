# Task 60: Add organization search by name

**Feature**: none
**Created**: 2026-01-31
**Depends on**: none
**Blocks**: Task 64

## Problem

Before importing a company, we need to check if it already exists to avoid duplicates. The import process should search organizations by name (case-insensitive, normalized).

Currently there's `get_organizations_for_person` but no direct search by organization name.

## Success criteria

- [ ] Function to search organizations by name (case-insensitive)
- [ ] Returns matching organization(s) or empty vec
- [ ] Optionally filter by city/state for stricter matching

## Notes

Normalization considerations for future improvement:
- Lowercase comparison
- Ignore suffixes like "LLC", "Inc", "Corp"
- Handle punctuation differences

For MVP, exact case-insensitive match is sufficient. Check `src/db/` for existing organization functions.

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
