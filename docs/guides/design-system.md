# Design System

Principles and patterns for contactcmd development.

## Core Principles

- **Minimal**: Only what's needed, nothing more
- **Clean**: Simplicity over cleverness
- **Reliable**: Same patterns everywhere
- **Fast**: Instant feedback, no delays
- **Antifragile**: Graceful degradation, handles edge cases
- **Composable**: Works in scripts and pipelines

## CLI Conventions

### Exit Codes

```rust
// Success
std::process::exit(0);

// General failure
std::process::exit(1);

// Usage error (bad arguments)
std::process::exit(2);
```

Always exit 0 on success, non-zero on failure. Scripts depend on this.

### Output Streams

```rust
// Normal output -> stdout
println!("John Smith");
println!("3 contacts synced.");

// Errors and warnings -> stderr
eprintln!("Error: contact not found");
eprintln!("Warning: could not sync");
```

Never mix errors into stdout. Piped output must be clean.

### TTY Detection

```rust
use std::io::IsTerminal;

if std::io::stdout().is_terminal() {
    // Interactive mode: colors, prompts, animations
    run_interactive()?;
} else {
    // Piped/scripted: plain output, no prompts
    run_batch()?;
}
```

Detect when output is piped and adjust behavior:
- Skip interactive prompts
- Disable progress animations
- Output plain text (no cursor movement)

### Signal Handling

```rust
// Ctrl+C (SIGINT): Exit immediately with cleanup
// Double Ctrl+C: Force exit, skip cleanup
// Design for crash-only: don't rely on cleanup running
```

The RAII pattern (RawModeGuard) handles most cleanup automatically.

### Standard Flags

| Flag | Purpose |
|------|---------|
| `--help`, `-h` | Show usage information |
| `--version`, `-V` | Show version |
| `--json` | Machine-readable output (future) |
| `--quiet`, `-q` | Suppress non-essential output |

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `NO_COLOR` | Disable colors (we have none, but respect it) |
| `CONTACTCMD_DB` | Override database path |
| `EDITOR` | Editor for notes |

## Configuration

### Hierarchy (highest wins)

```
1. Command-line flags      --db /path/to/db
2. Environment variables   CONTACTCMD_DB=/path/to/db
3. Config file             ~/.config/contactcmd/config.toml
4. XDG defaults            ~/.local/share/contactcmd/contacts.db
```

### XDG Base Directory Compliance

| Purpose | Path | Env Override |
|---------|------|--------------|
| Config | `~/.config/contactcmd/` | `XDG_CONFIG_HOME` |
| Data | `~/.local/share/contactcmd/` | `XDG_DATA_HOME` |
| Cache | `~/.cache/contactcmd/` | `XDG_CACHE_HOME` |

```rust
fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("contactcmd")
}
```

### Config File Format

```toml
# ~/.config/contactcmd/config.toml
[database]
path = "~/Dropbox/contacts.db"  # Optional override

[sync]
source = "macos"  # macos | google | carddav
auto_sync = false

[display]
page_size = 20
```

## Error Recovery

### Graceful Degradation

```rust
// Pattern: Try, warn, continue
match get_last_message_for_phones(&phones) {
    Ok(Some(msg)) => show_message(msg),
    Ok(None) => {},  // No message, not an error
    Err(e) => {
        // Log but don't fail the whole operation
        eprintln!("Warning: Could not read messages: {}", e);
    }
}
```

### Degradation Hierarchy

| Failure | Response |
|---------|----------|
| Messages DB locked | Show contact without messages, warn |
| Contacts permission denied | Show error, suggest fix, exit 1 |
| Network unavailable | Use cached data, warn |
| Disk full | Abort write, preserve existing data |
| Terminal too narrow | Truncate gracefully, never wrap mid-word |

### Idempotent Operations

```rust
// Safe to run multiple times
fn sync_contact(db: &Database, contact: &Contact) -> Result<()> {
    // Use UPSERT pattern - insert or update
    db.upsert_person(&contact.to_person())?;
    Ok(())
}
```

### Crash Recovery

Design for crash-only software:
- Never rely on cleanup running
- Use transactions for atomic operations
- Store state that survives restart

```rust
// Transaction ensures all-or-nothing
let tx = db.conn.transaction()?;
tx.execute("DELETE FROM emails WHERE person_id = ?", [id])?;
tx.execute("DELETE FROM phones WHERE person_id = ?", [id])?;
tx.execute("DELETE FROM persons WHERE id = ?", [id])?;
tx.commit()?;  // Only now is it committed
```

## Data Integrity

### Backup Strategy

```rust
// Before destructive operations
fn backup_database(db_path: &Path) -> Result<PathBuf> {
    let backup_path = db_path.with_extension("db.bak");
    std::fs::copy(db_path, &backup_path)?;
    Ok(backup_path)
}
```

### Validation Boundaries

Validate at system boundaries, trust internal data:

```rust
// Validate user input
fn parse_email(input: &str) -> Result<Email> {
    if !input.contains('@') {
        return Err(anyhow!("Invalid email format"));
    }
    Ok(Email::new(input))
}

// Trust data from our own database
fn get_email(db: &Database, id: Uuid) -> Result<Email> {
    // No validation needed - we wrote it
    db.get_email(id)
}
```

### Schema Migrations

```rust
const SCHEMA_VERSION: i32 = 2;

fn migrate(conn: &Connection) -> Result<()> {
    let current: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    if current < 1 {
        conn.execute_batch(include_str!("migrations/001_initial.sql"))?;
    }
    if current < 2 {
        conn.execute_batch(include_str!("migrations/002_add_tags.sql"))?;
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}
```

## Debug & Diagnostics

### Verbosity Levels

| Flag | Level | Output |
|------|-------|--------|
| (none) | Normal | Results only |
| `-v` | Verbose | + Progress info |
| `-vv` | Debug | + Internal state |
| `--debug` | Trace | + All queries, timing |

### Debug Output Pattern

```rust
fn run_sync(db: &Database, verbose: bool) -> Result<()> {
    if verbose {
        eprintln!("Connecting to macOS Contacts...");
    }

    let contacts = fetch_contacts()?;

    if verbose {
        eprintln!("Found {} contacts", contacts.len());
    }

    // Normal output to stdout
    println!("{} contacts synced.", contacts.len());
    Ok(())
}
```

### Diagnostic Command

```bash
contactcmd --doctor
# Database: OK (142 contacts)
# Messages: OK (accessible)
# macOS Contacts: OK (permission granted)
# Config: ~/.config/contactcmd/config.toml
# Data: ~/.local/share/contactcmd/contacts.db
```

## UI Patterns

See [docs/designs/ui-system.md](/docs/designs/ui-system.md) for complete visual specifications:
- Text hierarchy and spacing
- Prompt and feedback formats
- Navigation conventions
- Screen mockups

## Code Style

### Naming

```rust
// Types: PascalCase
struct ContactDetail { }
enum PhoneType { }

// Functions/methods: snake_case, verb first
fn get_contact_detail() { }
fn insert_person() { }
fn compute_names() { }

// Constants: SCREAMING_SNAKE_CASE
const MENU_OPTIONS: &[&str] = &["List", "Search"];

// Booleans: is_, has_, can_, should_
is_primary: bool
is_from_me: bool
has_changes: bool
```

### Error Handling

```rust
// Use anyhow::Result for fallible functions
pub fn run_show(db: &Database, id: &str) -> Result<()> { }

// Propagate with ?
let person = db.get_person(id)?;

// Provide context for unclear errors
let conn = Connection::open(&path)
    .context("Failed to open database")?;

// Use Option for expected absence
fn get_person(&self, id: Uuid) -> Result<Option<Person>> { }
```

### Function Design

```rust
// Small, focused functions
fn normalize_phone(phone: &str) -> String { }

// Early returns for clarity
if identifier.is_empty() {
    return Err(anyhow!("Identifier cannot be empty."));
}

// Avoid deep nesting
let Some(person) = result else {
    return Ok(None);
};
```

### Resource Safety

```rust
// RAII for cleanup (raw mode, file handles)
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

// Use guard pattern
let _guard = RawModeGuard::new()?;
// ... raw mode operations
// automatically cleaned up on scope exit
```

## Inquire Patterns

### Global Configuration

```rust
// Set once at startup for consistency
use inquire::set_global_render_config;

fn main() {
    set_global_render_config(minimal_render_config());
    // ...
}
```

### Input Validation

```rust
use inquire::validator::{Validation, StringValidator};
use inquire::required;

// Built-in validators
Text::new("email:")
    .with_validator(required!("Email is required"))
    .prompt()?;

// Custom validator
fn validate_email(input: &str) -> Result<Validation, CustomUserError> {
    if input.contains('@') {
        Ok(Validation::Valid)
    } else {
        Ok(Validation::Invalid("Must contain @".into()))
    }
}
```

### Skippable Prompts

```rust
// Use prompt_skippable() to handle Escape gracefully
let result = Text::new("name:")
    .prompt_skippable()?;

match result {
    Some(value) => process(value),
    None => return Ok(()), // User cancelled
}
```

### TTY Check Before Prompts

```rust
// Only prompt in interactive mode
if !std::io::stdin().is_terminal() {
    return Err(anyhow!("Interactive input required. Use flags instead."));
}
```

## File Organization

### Module Structure

```
src/
├── main.rs           # Entry point only
├── lib.rs            # Public exports
├── cli/
│   ├── mod.rs        # CLI commands
│   ├── ui.rs         # Shared UI primitives
│   ├── display.rs    # Contact formatting
│   └── [command].rs  # One file per command
├── db/
│   ├── mod.rs        # Database connection
│   ├── schema.rs     # SQL (source of truth)
│   └── persons.rs    # CRUD operations
└── models/
    └── [entity].rs   # One file per entity
```

### Test Organization

```rust
// Tests at bottom of file
#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Database {
        Database::open_memory().unwrap()
    }

    #[test]
    fn test_specific_behavior() {
        let db = setup_test_db();
        // ...
    }
}
```

## Database Patterns

### Query Style

```rust
// SQL as const strings for reuse
const SELECT_PERSON: &str = "SELECT * FROM persons WHERE id = ?";

// Use ? placeholders, never string interpolation
stmt.execute(params![id])?;

// Return Option for single-row queries
pub fn get_person(&self, id: Uuid) -> Result<Option<Person>> { }

// Return Vec for multi-row queries
pub fn list_persons(&self, limit: u32) -> Result<Vec<Person>> { }
```

### Transaction Safety

```rust
// Batch operations in transactions
let tx = self.conn.transaction()?;
for item in items {
    tx.execute(INSERT_SQL, params![item.id, item.name])?;
}
tx.commit()?;
```

## Testing Guidelines

### What to Test

- Public API behavior
- Edge cases (empty input, missing data)
- Error conditions
- Database operations

### What Not to Test

- Private implementation details
- UI rendering (manual testing)
- Third-party library behavior

### Test Naming

```rust
#[test]
fn test_[function]_[scenario]() { }

// Examples
fn test_search_no_results() { }
fn test_delete_person_cascade() { }
fn test_empty_identifier_error() { }
```

## Documentation

### When to Comment

```rust
// DO: Explain why, not what
// Apple uses nanoseconds since 2001-01-01, not Unix epoch
let seconds = timestamp / 1_000_000_000 + APPLE_EPOCH_OFFSET;

// DON'T: State the obvious
// Increment counter  <- bad
counter += 1;
```

### Module Documentation

```rust
//! Brief description of module purpose.
//!
//! Design principles:
//! - Key principle one
//! - Key principle two
```

## Commit Messages

```
<type>: <short description>

<optional body explaining why>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

---

References:
- [Command Line Interface Guidelines](https://clig.dev/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [inquire documentation](https://docs.rs/inquire/)
