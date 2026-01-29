# Task 41: Browse option for all contacts

**Feature**: /docs/features/contacts.md
**Created**: 2026-01-28
**Depends on**: Task 39 (photos)
**Blocks**: none

## Problem

Users want to casually flip through their contacts like a Rolodex - seeing full details and photos without the overhead of search or the commitment of review mode. The current list view shows a compact table, and the review mode implies you're working through contacts systematically. Sometimes you just want to browse.

## Success criteria

- [x] User can run `contactcmd browse` to start browsing all contacts
- [x] User can run `contactcmd browse --missing-email` to browse only contacts without email
- [x] User can run `contactcmd browse --missing-phone` to browse only contacts without phone
- [x] User can run `contactcmd browse --search "term"` to browse search results
- [x] Full contact details displayed (same as `show` command) including photo
- [x] Arrow left/right navigates, position shows "9/284"
- [x] Actions: `e` edit, `m` messages, `d` delete, `q` quit

## Behavior

The existing `list --review` mode already has the right UX:

```
9/284  [e]dit [m]essages [d]elete [←/→] [q]uit:
```

This task exposes that as a standalone `browse` command and adds filtered views.

### Filtered views

**Missing email** - Browse contacts that have no email address. Useful for data cleanup.

**Missing phone** - Browse contacts that have no phone number.

**Search results** - Browse contacts matching a search term instead of showing a list. This lets you flip through matches one at a time with full details and photos.

## Implementation approach

The existing `run_review_mode` function in `src/cli/list.rs:363` has the navigation and display logic. It currently fetches all persons internally via `db.list_persons_sorted()`. This task:

1. Adds a `browse` command that calls the same code
2. Refactors `run_review_mode` to accept a `Vec<Person>` instead of fetching internally
3. Uses existing database queries for filtered lists

The database already has the needed queries in `src/db/persons.rs`:
- `find_persons_missing_email(limit)` - line 419
- `find_persons_missing_phone(limit)` - line 402
- `find_persons_missing_both(limit)` - line 436
- `search_persons_multi(words, case_sensitive, limit)` - line 299

## Files to modify

- `src/cli/mod.rs` - Add `Browse` command variant and `BrowseArgs` struct with filter flags
- `src/main.rs` - Add browse command handler that builds the person list and calls browse mode
- `src/cli/list.rs` - Refactor `run_review_mode` to accept `Vec<Person>` parameter; export it as `pub`

## Notes

Keep `list --review` as an alias for backwards compatibility. Both `list --review` and `browse` call the same underlying function with the full contact list.
