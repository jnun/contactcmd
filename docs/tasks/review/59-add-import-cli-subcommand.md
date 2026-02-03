# Task 59: Add import CLI subcommand

**Feature**: none
**Created**: 2026-01-31
**Depends on**: none
**Blocks**: Task 64

## Problem

Users need a way to bulk import contacts from CSV files. Currently contactcmd only supports adding contacts one at a time via `contactcmd add` or syncing from macOS Contacts.

We need a new CLI subcommand: `contactcmd import <file>` with options for dry-run mode and source labeling.

## Success criteria

- [ ] `contactcmd import --help` shows usage
- [ ] `contactcmd import prospects.csv` is recognized (even if handler not yet functional)
- [ ] `--dry-run` flag available
- [ ] `--source <label>` flag available for tracking import origin

## Notes

Follow existing CLI patterns in `src/cli/`. The command will be wired to the actual import logic in Task 64.

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
