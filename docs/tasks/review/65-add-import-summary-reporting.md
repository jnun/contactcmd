# Task 65: Add import summary reporting

**Feature**: none
**Created**: 2026-01-31
**Depends on**: Task 64
**Blocks**: none

## Problem

After import completes, users need a clear summary of what happened. How many records imported? How many skipped? Any errors?

## Success criteria

- [ ] Import prints summary: "Imported X organizations, Y contacts"
- [ ] Shows duplicates skipped: "Skipped Z duplicates"
- [ ] Shows errors if any occurred
- [ ] Dry-run mode shows: "Would import X organizations (dry run)"

## Notes

**Location:** `src/cli/import.rs`

**Existing function signature (from Task 59):**
```rust
pub fn run_import(
    db: &Database,
    file: &str,
    dry_run: bool,
    source: Option<&str>,
) -> Result<()>
```

The `dry_run` parameter controls whether to actually insert or just preview.

**Example output:**
```
Importing prospects.csv...
[====================================] 4923/4923

Import complete:
  ✓ 4800 organizations created
  ✓ 4800 contacts created
  ✓ 545 with websites
  ✓ 311 with emails
  - 123 duplicates skipped
```

Consider using the `indicatif` crate for progress bars (already in project for other commands).

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
