# Task 42: Clean up navigation

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Task 41 (browse)
**Blocks**: none

## Problem

The CLI has accumulated commands that duplicate functionality now available through search and browse. Users have to remember multiple ways to do the same thing. Meanwhile, the messages search is hobbled by an implementation that only looks at recent messages instead of searching the full history.

## Commands to remove

### `update`

**Why redundant**: Search finds a contact and shows `[e]dit` option. Same with `show` and `browse`. There's no scenario where typing `contactcmd update "John Smith" -e new@email.com` is better than searching for John and pressing `e`.

**Action**: Remove the `update` command entirely.

### `delete`

**Why redundant**: Search, show, and browse all have `[d]elete` option. The standalone delete command adds no value.

**Action**: Remove the `delete` command entirely.

### `note`

**Why redundant**: Search and browse have `[n]otes` action. The quick-note workflow (`contactcmd note "John" "Called about project"`) is nice but rarely used compared to finding someone first then adding context.

**Action**: Remove the `note` command entirely.

## Commands to fix

### `messages`

**Current problem**: Searches only the last 5000 messages fetched into memory, then filters. If your search term appears in older messages, you'll never find them. A search for "music" among years of texts shows almost nothing.

**The fix**: Query the Messages database directly with SQL filtering:
```
WHERE text LIKE '%music%'
```
This searches the full history, not just recent messages.

**Additional improvements**:
- Remove the 200 result cap (or make it much higher)
- Add date range filtering (`--since 2024-01-01`)
- Show match count per contact in the header

**Action**: Keep the command but fix the search to query full history.

## Success criteria

- [ ] `update` command removed - running it shows "unknown command"
- [ ] `delete` command removed - running it shows "unknown command"
- [ ] `note` command removed - running it shows "unknown command"
- [ ] `messages "music"` finds matches from years ago, not just recent
- [ ] `--help` shows cleaner command list

## Resulting command set

After cleanup:

| Command | Purpose |
|---------|---------|
| `list` | Table view of all contacts |
| `browse` | Full-detail view, flip through with arrows |
| `search` | Find contacts by name/email/notes/etc |
| `show` | View one contact's full details |
| `add` | Create new contact |
| `photo` | Set contact photo |
| `messages` | Search full iMessage history |
| `sync` | Import from macOS Contacts |

Eight commands instead of eleven. Each does one thing well.

## Files to modify

**Remove:**
- `src/cli/update.rs` - delete file
- `src/cli/delete.rs` - delete file
- `src/cli/note.rs` - delete file

**Update:**
- `src/cli/mod.rs` - remove Update, Delete, Note from Commands enum
- `src/main.rs` - remove match arms for removed commands
- `src/cli/messages/macos.rs` - rewrite search to query full database

## Notes

The delete functionality remains available via `[d]` in search/browse/show. We're removing the standalone command, not the capability.

Consider keeping `delete` with `--force` for scripting use cases. But probably not needed - if you're scripting, you'd use the database directly.
