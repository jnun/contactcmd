# Architecture

Layered Rust CLI application.

## Structure

```
src/
├── main.rs              # Entry point, command dispatch
├── lib.rs               # Public exports
├── cli/mod.rs           # CLI definition (clap)
├── db/
│   ├── mod.rs           # Database connection, migrations
│   ├── schema.rs        # SQL schema (source of truth)
│   └── persons.rs       # CRUD operations
├── models/              # Domain structs (Person, Email, Phone, etc.)
├── sync/mod.rs          # External sync (macOS Contacts) - todo
└── ui/mod.rs            # Terminal UI - todo
```

## Key Decisions

### UUIDs as Primary Keys
Stored as TEXT in SQLite. Enables offline-first sync without ID conflicts.

### Computed Name Fields
`Person` has three computed fields (call `compute_names()` after changes):
- `display_name` - "John Smith" or "Tanaka Taro"
- `sort_name` - "Smith, John"
- `search_name` - lowercase, all name parts

### Soft Delete
- `delete_person()` - Hard delete (CASCADE removes related records)
- `deactivate_person()` - Soft delete (sets `is_active = false`)
- `reactivate_person()` - Undo soft delete

### Auto Timestamps
`update_person()` automatically sets `updated_at` to now.

### Error Handling
Uses `anyhow::Result`. Database errors propagate properly (no silent drops).

## Dependencies

| Crate | Purpose |
|-------|---------|
| clap | CLI parsing |
| rusqlite | SQLite (bundled) |
| uuid | UUIDs |
| chrono | DateTime |
| serde | Serialization |
| anyhow/thiserror | Errors |
| dirs | Config paths |
| crossterm | Terminal UI (future) |
| objc2-contacts | macOS sync (future) |
