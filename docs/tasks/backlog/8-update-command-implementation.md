# Task 8: update Command Implementation

**Feature:** /docs/features/cmd-update.md
**Created:** 2026-01-27
**Depends on:** Task 3

## Problem

Implement the `contactcmd update` command to modify existing contacts interactively or via CLI options.

## Success criteria

- [ ] `update <id>` by UUID
- [ ] `update "name"` by name search
- [ ] Interactive menu for editing fields
- [ ] Direct mode with CLI options
- [ ] Updates any field (name, email, phone, org, notes)
- [ ] `--add-email`, `--add-phone` for additional records
- [ ] `--add-tag`, `--remove-tag` for tags
- [ ] Shows what changed after update
- [ ] Sets is_dirty=true for sync tracking
- [ ] Recomputes display_name/sort_name/search_name if name changed

## Notes

See /docs/features/cmd-update.md for full specification.

Options:
- `-f, --first` First name
- `-l, --last` Last name
- `-e, --email` Update primary email
- `-p, --phone` Update primary phone
- `-c, --company` Company
- `-t, --title` Job title
- `-n, --notes` Notes
- `--add-email`, `--add-phone`, `--add-tag`, `--remove-tag`
