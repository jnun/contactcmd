# Task 36: Quick Note Command

**Feature**: none
**Created**: 2026-01-28

## Problem

Adding notes to contacts requires multiple steps: search → show → edit → navigate to notes → type → save. After a call or meeting, users want to capture notes while context is fresh.

## Success criteria

- [x] New `note` subcommand in CLI
- [x] Syntax: `contactcmd note <search> [note text...]`
- [x] First argument is search term, remaining arguments are note text
- [x] Quotes required for multi-word search: `note "aaron andrews" Called him`
- [x] If 1 match: directly add note
- [x] If multiple matches: inquire Select to choose contact
- [x] If no matches: `No matches.`
- [x] If note text provided: append immediately
- [x] If no note text: prompt with inquire Text
- [x] Timestamp format: `[2026-01-28 14:30 CST] note text`
- [x] Append to existing notes (newline separated), not replace
- [x] Success feedback: `Saved.`
- [x] Add "Note" option to main menu

## Technical details

**Storage:**
- Timestamp stored as UTC
- Displayed as machine local time with timezone abbreviation (CST/CDT)
- Note appended to `Person.notes` field in SQLite database

**Files to modify:**
- `src/cli/mod.rs` - Add `Note` command variant and `NoteArgs` struct
- `src/cli/note.rs` - New file for `run_note()` implementation
- `src/cli/menu.rs` - Add "Note" option to main menu
- `src/main.rs` - Add match arm for Note command

**Existing patterns to follow:**
- Search: `db.search_persons_multi()` (see search.rs)
- Selection: inquire Select (see menu.rs)
- Update person: `db.update_person()` (see update.rs)

## Notes

**Argument parsing:**
```
note aaron Called about project     → search "aaron", note "Called about project"
note "aaron andrews" Called today   → search "aaron andrews", note "Called today"
note aaron                          → search "aaron", prompt for note
```

**Example flows:**

Single match with inline note:
```
$ contactcmd note aaron Called about Q2 plans
Saved.
```

Multiple matches:
```
$ contactcmd note aaron

> Aaron Andrews (aaron@acme.com)
  Aaron Smith (asmith@gmail.com)

note: Called about Q2 plans
Saved.
```

**Future (not in scope):**
- Calendar integration to auto-suggest contact from recent meeting
- `--last` flag to add note to most recently viewed contact

---
