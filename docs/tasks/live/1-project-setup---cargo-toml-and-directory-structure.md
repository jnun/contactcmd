# Task 1: Project Setup

**Feature:** /docs/features/contactcmd-core.md
**Created:** 2026-01-27
**Blocks:** Tasks 2-10

## User Story

As a developer, I need to initialize the Rust project with proper structure and dependencies so that subsequent tasks have a working foundation to build on.

## Acceptance Criteria

- [x] Cargo.toml with required dependencies:
  - clap (CLI parsing with derive)
  - rusqlite (SQLite database with bundled)
  - chrono (date/time handling)
  - uuid (unique identifiers)
  - serde + serde_json (serialization)
  - crossterm (terminal UI)
  - anyhow + thiserror (error handling)
  - dirs (platform directories)
  - macOS: objc2 + objc2-contacts (Contacts framework)
- [x] Module structure: cli/, db/, models/, sync/, ui/
- [x] CLI skeleton with subcommands: list, search, show, add, update, delete, sync
- [x] `cargo build` succeeds
- [x] `cargo test` runs
- [x] `contactcmd --help` displays all subcommands

## References

- /docs/features/contactcmd-core.md for command descriptions
