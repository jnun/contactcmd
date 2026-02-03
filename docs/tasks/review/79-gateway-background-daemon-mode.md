# Task 79: Gateway background daemon mode

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: none
**Blocks**: none

## Problem

Currently `contactcmd gateway start` requires `--foreground` flag because background mode isn't implemented. Users must keep a terminal open or manually manage backgrounding. A proper daemon mode would fork, detach from terminal, and run until stopped.

## Success criteria

- [x] `contactcmd gateway start` (without --foreground) daemonizes properly
- [x] Process detaches from controlling terminal
- [x] PID written to `~/.config/contactcmd/gateway.pid` after fork
- [x] Logs written to `~/.config/contactcmd/gateway.log`
- [x] `contactcmd gateway stop` sends SIGTERM to daemon
- [x] `contactcmd gateway status` correctly reports daemon running/stopped

## Notes

- Use `daemonize` crate or manual fork/setsid
- Consider log rotation or size limits
- macOS: could also provide launchd plist in future
- Linux: could also provide systemd unit in future

## Implementation

- Added `daemonize` crate (v0.5) to Cargo.toml
- Modified `start_gateway()` in `src/cli/gateway/mod.rs` to use `Daemonize` builder
- Added `log_file_path()` helper function
- Updated `show_status()` to display log file path when running
- Daemon redirects stdout/stderr to gateway.log with timestamped entries
