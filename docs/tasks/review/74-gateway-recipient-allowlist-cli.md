# Task 74: Gateway recipient allowlist CLI

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: Task 72
**Blocks**: none

## Problem

Users need a way to manage allowlists for their API keys. Without CLI commands, they'd have to manually edit the database.

## Success criteria

- [x] `contactcmd gateway keys allowlist add <key-id> <pattern>` adds allowlist entry
- [x] `contactcmd gateway keys allowlist list <key-id>` shows current allowlist
- [x] `contactcmd gateway keys allowlist remove <key-id> <pattern>` removes entry
- [x] Commands accept key ID prefix (e.g., `gw_abc123`) not just full UUID
- [x] Adding duplicate pattern is idempotent (no error)

## Notes

- Could nest under existing `keys` subcommand
- Show helpful examples in `--help` output
- Consider `gateway keys show <id>` to display key details + allowlist together

## Implementation

- Added `AllowlistCommands` enum in `src/cli/gateway/mod.rs`:
  - `Set` - add pattern (with `add` alias)
  - `List` - show patterns for a key
  - `Remove` - delete pattern
- Added `Allowlist` variant to `KeysCommands`
- Added helper function `find_key_by_prefix()` for fuzzy key matching
- Added handler functions:
  - `allowlist_add()` - idempotent insert, shows current list after
  - `allowlist_list()` - shows patterns with dates, helpful examples when empty
  - `allowlist_remove()` - shows remaining list after deletion
- CLI help includes pattern examples in argument descriptions

## Usage Examples

```bash
# Add patterns to allowlist
contactcmd gateway keys allowlist set abc123 'john@example.com'
contactcmd gateway keys allowlist set gw_xyz '*@acme.com'
contactcmd gateway keys allowlist add abc123 '+15551234567'

# List allowlist
contactcmd gateway keys allowlist list abc123

# Remove pattern
contactcmd gateway keys allowlist remove abc123 '*@acme.com'
```
