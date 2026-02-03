# Task 58: Add csv crate dependency

**Feature**: none
**Created**: 2026-01-31
**Depends on**: none
**Blocks**: Task 62

## Problem

We're building a CSV import feature for contactcmd to bulk import contractor/prospect data from external sources. The first data source is Alabama contractor licenses (4,923 records with company names, addresses, phones, websites, emails).

Rust needs the `csv` crate to parse CSV files. This is a foundational dependency for the import feature.

## Success criteria

- [ ] `csv` crate added to Cargo.toml
- [ ] `cargo build` succeeds with the new dependency

## Notes

Use `csv = "1.3"` or latest stable version. The serde integration (`csv` + `serde`) is useful for deserializing directly into structs.

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
