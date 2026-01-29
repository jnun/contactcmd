# Task 7: add Command Implementation

**Feature:** /docs/features/cmd-add.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement the `contactcmd add` command to create new contacts interactively or via CLI options.

## Success criteria

- [x] Interactive mode prompts for all fields
- [x] Direct mode with CLI options (-f, -l, -e, -p, -n)
- [x] At least first or last name required
- [x] Email format validated
- [x] Duplicate detection warns if similar contact exists
- [x] Creates person record with UUID
- [x] Creates related email, phone records
- [ ] Creates/links organization if company provided (deferred - db method doesn't exist)
- [x] Returns UUID on success
- [x] `cargo test cli::add` passes (5 tests)

## Verification

```
$ cargo run -- add --help
Add a new contact

Usage: contactcmd add [OPTIONS]

Options:
  -f, --first <FIRST>
  -l, --last <LAST>
  -e, --email <EMAIL>
  -p, --phone <PHONE>
  -c, --company <COMPANY>
  -t, --title <TITLE>
  -n, --notes <NOTES>
  -h, --help               Print help

$ cargo test cli::add
running 5 tests
test cli::add::tests::test_add_first_name_only ... ok
test cli::add::tests::test_add_invalid_email ... ok
test cli::add::tests::test_add_person_direct ... ok
test cli::add::tests::test_add_requires_name ... ok
test cli::add::tests::test_valid_email ... ok
test result: ok. 5 passed
```

## Notes

- Organization support deferred: `--company` and `--title` flags exist in CLI but aren't wired up because `db.insert_organization()` doesn't exist yet
- Interactive mode triggers when no options provided
- Duplicate detection checks both name match and email match
