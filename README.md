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

# Search iMessage history
contactcmd messages "lunch"
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

### messages
```bash
contactcmd messages "lunch"                # Search full iMessage history
contactcmd messages "project" --since 2024-01-01  # With date filter
```

### sync
```bash
contactcmd sync mac                 # Import from macOS Contacts
contactcmd sync mac --dry-run       # Preview without importing
```

### gateway
```bash
contactcmd gateway start            # Start gateway server (port 9810)
contactcmd gateway stop             # Stop gateway server
contactcmd gateway status           # Show status + pending count
contactcmd gateway approve          # TUI for reviewing queued messages

contactcmd gateway keys add "Agent" # Generate API key for an agent
contactcmd gateway keys list        # List all API keys
contactcmd gateway keys revoke <id> # Revoke an API key
```

## AI Agent Gateway

The gateway provides human-in-the-loop approval for AI-initiated messages, preventing agents from impersonating you to trusted contacts.

```
AI Agent ──► Gateway API ──► Approval Queue ──► Human Review ──► Send
```

**Setup:**
1. Start the gateway: `contactcmd gateway start --foreground`
2. Generate an API key: `contactcmd gateway keys add "My AI Agent"`
3. Configure your agent with the gateway URL and API key
4. Review queued messages: `contactcmd gateway approve` (or select "Gateway" from main menu)

**API Endpoints:**
- `POST /gateway/send` - Queue a message (requires API key)
- `GET /gateway/actions/{id}` - Poll message status
- `GET /gateway/health` - Health check

See [Gateway docs](docs/features/gateway.md) for full API reference.

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
| Database schema (V7) | Complete |
| Models & CRUD | Complete |
| list command | Complete |
| search command | Complete |
| show command | Complete |
| add command | Complete |
| messages command | Complete |
| macOS sync | Complete |
| Gateway (AI approval queue) | Complete |
| Email sending (Gmail) | Complete |
| SMS/iMessage sending | Complete |

## Development

```bash
cargo build          # Build
cargo test           # Run tests (171 tests)
cargo run -- --help  # Run in development
```

## License

Private, for distribute by 5DayApp
