# Task 19: Show command has hardcoded 20-result limit

**Feature**: /docs/features/cmd-show.md
**Created**: 2026-01-27

## Problem

The `show` command searches with a hardcoded limit of 20 results. Users can't access results beyond #20 and don't know results were truncated.

## Success criteria

- [x] `show` command accepts `--limit` flag (default 20, same as search)
- [x] When results are truncated, show a message: "Showing first N of M+ matches. Use --limit to see more."
- [x] Update `ShowArgs` in `cli/mod.rs` to include limit field
- [x] Update `run_show()` to accept and use the limit parameter
- [x] `cargo test cli::show` passes (6 tests)
- [x] `cargo run -- show --help` shows the new flag

## Verification

```
$ cargo run -- show --help
Show full details for a contact

Usage: contactcmd show [OPTIONS] <IDENTIFIER>

Arguments:
  <IDENTIFIER>

Options:
  -l, --limit <LIMIT>  [default: 20]
  -h, --help           Print help

$ cargo test cli::show
running 6 tests
test cli::show::tests::test_empty_identifier_error ... ok
test cli::show::tests::test_search_multiple_matches ... ok
test cli::show::tests::test_search_no_match ... ok
test cli::show::tests::test_search_single_match ... ok
test cli::show::tests::test_show_by_uuid ... ok
test cli::show::tests::test_truncation_detection ... ok
test result: ok. 6 passed
```
