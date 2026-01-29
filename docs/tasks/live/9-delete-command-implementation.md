# Task 9: delete Command Implementation

**Feature:** /docs/features/cmd-delete.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement the `contactcmd delete` command to remove contacts with confirmation.

## Success criteria

- [x] `delete <uuid>` deletes by ID
- [x] `delete "name"` searches and deletes by name
- [x] Shows contact details before confirmation
- [x] Requires typing 'yes' to confirm
- [x] `--force` flag skips confirmation
- [ ] `--batch` handles comma-separated list (deferred - separate task)
- [x] Multiple name matches show selection menu
- [x] Cascade deletes all related records (emails, phones, etc.)
- [x] Returns success/failure message
- [x] `cargo test cli::delete` passes (4 tests)

## Verification

```
$ cargo run -- delete --help
Delete a contact

Usage: contactcmd delete [OPTIONS] <IDENTIFIER>

Arguments:
  <IDENTIFIER>

Options:
  -f, --force
  -h, --help   Print help

$ cargo test cli::delete
running 4 tests
test cli::delete::tests::test_cascade_delete ... ok
test cli::delete::tests::test_delete_by_uuid_force ... ok
test cli::delete::tests::test_delete_nonexistent ... ok
test cli::delete::tests::test_empty_identifier_error ... ok
test result: ok. 4 passed
```

## Notes

- `--batch` flag deferred to separate task (adds complexity)
- Cascade delete handled by SQLite foreign key constraints
