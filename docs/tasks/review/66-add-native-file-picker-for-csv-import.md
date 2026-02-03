# Task 66: Add native file picker for CSV import

**Feature**: /docs/features/csv-import.md
**Created**: 2026-01-31
**Depends on**: Task 59
**Blocks**: none

## Problem

Users had to type full CSV path manually. Should open native file picker.

## Success criteria

- [x] Add `rfd` crate dependency to Cargo.toml
- [x] Create `pick_csv_file()` function with CSV filter
- [x] File argument optional in import CLI
- [x] No path provided opens file picker
- [x] Cancel exits gracefully
- [x] Selected path passed to `run_import`

## Done

`contactcmd import` opens Finder. `contactcmd import file.csv` uses provided path.
