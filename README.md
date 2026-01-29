# ContactCMD

A fast, portable personal CRM for the command line. Built in Rust.

** AI assisted deterministic human gated controls for direct communication. **

## Installation

```bash
# Build and install
cargo install --path .

# Or build without installing
cargo build --release
./target/release/contactcmd --help
```

## Quick Start

```bash
# Import contacts from macOS Contacts app
contactcmd sync mac

# List all contacts (interactive paginated view)
contactcmd list

# Search for contacts
contactcmd search john

# View full contact details
contactcmd show "John Smith"

# Add a new contact
contactcmd add -f John -l Smith -e john@example.com -p 555-1234

# Update a contact
contactcmd update "John Smith" -e newemail@example.com

# Delete a contact
contactcmd delete "John Smith"
```

## Commands

### list
```bash
contactcmd list                     # Interactive paginated view
contactcmd list --all               # Print all contacts (non-interactive)
contactcmd list --sort updated      # Sort by updated_at
contactcmd list --order desc        # Descending order
contactcmd list --page 2 --limit 50 # Pagination controls
```

### search
```bash
contactcmd search john              # Case-insensitive search
contactcmd search "john smith"      # Multi-word AND search
contactcmd search john --limit 50   # Limit results
contactcmd search John -c           # Case-sensitive search
```

### show
```bash
contactcmd show "John Smith"        # Show by name
contactcmd show a5f2ea15-...        # Show by UUID
contactcmd show john --limit 50     # Limit interaction history
```

### add
```bash
contactcmd add                      # Interactive mode
contactcmd add -f John -l Smith     # Direct mode with options
contactcmd add -f John -e j@x.com -p 555-1234 -n "Met at conference"
```

### update
```bash
contactcmd update "John Smith" -f Johnny           # Update first name
contactcmd update "John Smith" -e new@example.com  # Update email
contactcmd update a5f2ea15-... -n "New notes"      # Update by UUID
```

### delete
```bash
contactcmd delete "John Smith"      # Delete with confirmation
contactcmd delete "John Smith" -f   # Force delete (no confirmation)
```

### sync
```bash
contactcmd sync mac                 # Import from macOS Contacts
contactcmd sync mac --dry-run       # Preview without importing
```

## macOS Contacts Sync

Import your contacts from the macOS Contacts app:

```bash
contactcmd sync mac
```

On first run, you'll be prompted to grant access to Contacts. If denied:
1. Open System Settings > Privacy & Security > Contacts
2. Enable access for Terminal (or your terminal app)
3. Run the command again

Features:
- Imports names, emails, phones, addresses, organizations, job titles, birthdays
- Tracks Apple Contact IDs for re-sync (updates instead of duplicates)
- Use `--dry-run` to preview changes before importing

## Requirements

- Rust 1.75+
- macOS 11+ (for Contacts sync)

## Data Storage

SQLite database at `~/.config/contactcmd/contacts.db`

## Documentation

- [Architecture](docs/guides/architecture.md) - System design and key decisions
- [Data Model](docs/guides/data-model.md) - Database schema and CRUD operations
- [Development](docs/guides/development-setup.md) - Build, test, contribute

## Project Status

| Feature | Status |
|---------|--------|
| Project setup | Complete |
| Database schema | Complete |
| Models & CRUD | Complete |
| list command | Complete |
| search command | Complete |
| show command | Complete |
| add command | Complete |
| update command | Complete |
| delete command | Complete |
| macOS sync | Complete |

## Development

```bash
cargo build          # Build
cargo test           # Run tests (51 tests)
cargo run -- --help  # Run in development
```

## License

Private, for distribute by 5DayApp
