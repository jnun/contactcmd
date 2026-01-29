# Task 18: Implement show command

**Feature**: /docs/features/cmd-show.md
**Created**: 2026-01-27
**Depends on**: Task 17

## Problem

Running `contactcmd show "John"` panics with `not yet implemented: show`. The CLI arguments are defined (`ShowArgs` in `cli/mod.rs:59-62`) but there's no handler - just `todo!("show")` in `main.rs:16`.

Users need to view full details of a contact by name or UUID.

## Success criteria

- [x] Create `src/cli/show.rs` with `run_show(&Database, &str)` function
- [x] UUID lookup: `show <uuid>` fetches contact directly
- [x] Name search: `show "name"` searches and displays matching contact
- [x] Multiple matches: show numbered selection menu (1-N, q to quit)
- [x] Selection menu uses already-fetched data (no N+1 queries)
- [x] Wire up in `main.rs`: replace `todo!("show")` with `run_show()` call
- [x] Export from `cli/mod.rs`: `pub use show::run_show;`
- [x] `cargo test cli::show` passes (5 tests)
- [x] `cargo run -- show --help` shows usage

## Verification

```
$ cargo test cli::show
running 5 tests
test cli::show::tests::test_empty_identifier_error ... ok
test cli::show::tests::test_search_multiple_matches ... ok
test cli::show::tests::test_search_no_match ... ok
test cli::show::tests::test_search_single_match ... ok
test cli::show::tests::test_show_by_uuid ... ok
test result: ok. 5 passed

$ cargo run -- show --help
Show full details for a contact
Usage: contactcmd show <IDENTIFIER>

$ cargo run -- show "nonexistent"
No contacts found matching "nonexistent".
Tip: Use 'contactcmd list' to browse all contacts.
```
