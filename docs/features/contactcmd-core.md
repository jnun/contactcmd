# Feature: ContactCMD Core

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

A fast, portable personal CRM for the command line. Manages contacts locally with SQLite storage and syncs with macOS Contacts.

## User Stories

- As a user, I want to list my contacts so I can browse who I know
- As a user, I want to search contacts so I can find someone quickly
- As a user, I want to view a contact's full details so I can see all their info
- As a user, I want to add contacts so I can track new people I meet
- As a user, I want to update contacts so I can keep info current
- As a user, I want to delete contacts so I can remove outdated entries
- As a user, I want to sync with macOS Contacts so I don't duplicate effort

## Requirements

### Functional Requirements

- [ ] Store contacts in local SQLite database
- [ ] Support multiple emails, phones, addresses per contact
- [ ] Track organizations and job history
- [ ] Support tags for categorization
- [ ] Track special dates (birthdays, anniversaries)
- [ ] Import from macOS Contacts

### Non-Functional Requirements

- [ ] Sub-100ms response for all local operations
- [ ] Single binary with no runtime dependencies
- [ ] Works offline
- [ ] Database under 50MB for 10,000 contacts

## Technical Design

### Architecture

```
contactcmd (Rust)
├── cli/        # Command parsing (clap)
├── db/         # SQLite operations (rusqlite)
├── models/     # Data structures
├── sync/       # macOS Contacts integration
└── ui/         # Terminal output formatting
```

### Dependencies

- clap: CLI argument parsing
- rusqlite: SQLite database
- uuid: Contact IDs
- chrono: Date/time handling
- crossterm: Terminal UI

### Data Model

Core entities:
- Person (central contact record)
- Email, Phone, Address (contact methods)
- Organization, PersonOrganization (work history)
- Tag, PersonTag (categorization)
- SpecialDate (birthdays, etc.)

## Implementation Tasks

Reference task IDs that implement this feature:
- [ ] Task #1 - Project setup (Cargo.toml, structure)
- [ ] Task #2 - Database schema and migrations
- [ ] Task #3 - Person model and CRUD
- [ ] Task #4 - list command
- [ ] Task #5 - search command
- [ ] Task #6 - show command
- [ ] Task #7 - add command
- [ ] Task #8 - update command
- [ ] Task #9 - delete command
- [ ] Task #10 - macOS sync

## Acceptance Criteria

- [ ] `contactcmd list` shows paginated contacts
- [ ] `contactcmd search "name"` finds matching contacts
- [ ] `contactcmd show <id>` displays full contact details
- [ ] `contactcmd add` creates new contact
- [ ] `contactcmd update <id>` modifies existing contact
- [ ] `contactcmd delete <id>` removes contact with confirmation
- [ ] `contactcmd sync mac` imports from macOS Contacts
- [ ] All commands complete in <100ms (local ops)

## Notes

Migrating from Python implementation. Existing data can be exported to JSON and re-imported.
