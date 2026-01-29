# Task 14: CardDAV Sync

**Feature:** /docs/features/cmd-sync.md
**Created:** 2026-01-27
**Depends on:** Task 3, Task 13

## Problem

Implement CardDAV sync to synchronize contacts with any CardDAV-compatible server (iCloud, Fastmail, Nextcloud, ownCloud, Zimbra, etc.). CardDAV is the standard protocol for contact synchronization over the network.

## Success criteria

- [ ] `contactcmd sync carddav` syncs with configured CardDAV server
- [ ] Server URL, username, credentials stored in config
- [ ] Secure credential storage in system keychain
- [ ] Auto-discovers address books via .well-known/carddav
- [ ] `--addressbook` flag to select specific address book
- [ ] Lists available address books with `--list-addressbooks`
- [ ] Downloads all contacts from server
- [ ] Parses vCard responses (reuses Task 13 vCard parser)
- [ ] Tracks ETags for change detection
- [ ] Tracks external_ids.carddav for contact matching
- [ ] Incremental sync using sync-token/ctag when supported
- [ ] `--dry-run` previews changes
- [ ] `--direction` flag: pull, push, or bidirectional
- [ ] Handles conflict resolution (server-wins default)
- [ ] Progress indicator
- [ ] Handles auth failures gracefully

## Notes

CLI interface:
```
contactcmd sync carddav [OPTIONS]
  --server URL      CardDAV server URL (or use config)
  --username USER   Username (or use config)
  --addressbook AB  Address book name/path
  --list-addressbooks  List available address books
  --dry-run         Preview changes
  --direction DIR   pull|push|sync (default: pull)
  --force           Overwrite conflicts
```

Configuration in ~/.config/contactcmd/config.toml:
```toml
[carddav]
server = "https://carddav.fastmail.com"
username = "user@example.com"
# password retrieved from keychain
addressbook = "Default"
```

Common CardDAV servers:
- iCloud: https://contacts.icloud.com
- Fastmail: https://carddav.fastmail.com
- Nextcloud: https://cloud.example.com/remote.php/dav
- Google: https://www.googleapis.com/carddav/v1/principals/user@gmail.com/lists/default

Protocol requires:
- PROPFIND for discovery
- REPORT for sync
- GET for vCard retrieval
- PUT for vCard upload
- DELETE for removal

Consider using reqwest + xml-rs or quick-xml for protocol handling.
