# Task 17: Create display module for contact formatting

**Feature**: /docs/features/cmd-show.md
**Created**: 2026-01-27
**Blocks**: Task 18

## Problem

The `list` and `search` commands each have their own inline display logic. The `show` command needs rich contact formatting, but there's no shared display module. Task 16 was supposed to create this but was marked "superseded by Task 4" - however Task 4 never actually created the module.

We need a shared `src/cli/display.rs` module with formatting functions that multiple commands can use.

## Success criteria

- [x] Create `src/cli/display.rs` with shared formatting functions
- [x] `print_full_contact(&ContactDetail)` - rich full-page display for show command
- [x] Functions take pre-fetched data (no database access inside display functions)
- [x] Add `pub mod display;` to `src/cli/mod.rs`
- [x] `cargo build` succeeds

## Notes

- `print_contact_row` remains in `list.rs` since `ContactListRow` is defined there and the list command already works
- Future refactoring could move `ContactListRow` to models and consolidate row formatting in display.rs
- See `/docs/features/cmd-show.md` for the expected full-contact output format
- `ContactDetail` struct exists at `src/models/contact_detail.rs` with helper methods

## Verification

```
$ cargo build
   Compiling contactcmd v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.55s

$ cargo test cli::display
running 2 tests
test cli::display::tests::test_capitalize ... ok
test cli::display::tests::test_print_full_contact_does_not_panic ... ok
test result: ok. 2 passed
```
