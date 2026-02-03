# Architecture

Layered Rust CLI application.

## Structure

```
src/
├── main.rs              # Entry point, command dispatch
├── lib.rs               # Public exports
├── cli/
│   ├── mod.rs           # CLI definition (clap)
│   ├── menu.rs          # Main menu TUI
│   ├── chat.rs          # Natural language chat interface
│   ├── show.rs          # Contact detail view + messaging
│   ├── search.rs        # Contact search
│   ├── list.rs          # Contact list/browse
│   ├── task.rs          # Task management TUI
│   ├── email.rs         # Email compose (Gmail API)
│   ├── ai/              # AI chat assistant
│   │   ├── mod.rs       # Module exports
│   │   ├── session.rs   # Chat session + feedback loop
│   │   ├── tools.rs     # Tool definitions for AI
│   │   ├── executor.rs  # Tool call → command translation
│   │   ├── instructions.md  # AI system prompt (compiled in)
│   │   ├── config.rs    # AI provider configuration
│   │   ├── remote.rs    # Remote API provider (OpenAI-compatible)
│   │   └── provider.rs  # Provider trait
│   ├── bridge/          # Moltbot bridge for iMessage relay
│   ├── gateway/         # AI agent communication gateway
│   │   ├── mod.rs       # CLI commands (start/stop/keys)
│   │   ├── server.rs    # HTTP API server
│   │   ├── types.rs     # Request/response types
│   │   ├── keys.rs      # API key management
│   │   ├── approve.rs   # Approval TUI
│   │   └── execute.rs   # Message sending
│   └── ui.rs            # Shared UI primitives
├── db/
│   ├── mod.rs           # Database connection, migrations
│   ├── schema.rs        # SQL schema (V1-V7)
│   ├── persons.rs       # Contact CRUD operations
│   └── gateway.rs       # Gateway queue + API key CRUD
├── models/              # Domain structs (Person, Email, Phone, Task, etc.)
└── sync/                # External sync (macOS Contacts)
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

## AI Chat Architecture

Natural language interface for contact management via `contactcmd chat`.

```
User Input ──► Chat Parser ──► AI Session ──► Tool Call ──► Command
                   │               │              │            │
                   │               │              ▼            ▼
                   │               │          Executor      Execute
                   │               │              │            │
                   │               ◄──── Feedback Loop ◄──────┘
                   │
                   └──► Fallback (if AI skips tool)
```

**Key design principles:**
- AI has NO access to user data (security firewall)
- AI only suggests commands via tools
- Commands auto-execute after AI responds
- Feedback loop handles search failures (suggests simpler search)
- Fallback parser catches cases where AI doesn't call tools

**Files:**
- `instructions.md` - System prompt (compiled into binary via `include_str!`)
- `session.rs` - Manages conversation, captures tool calls, handles feedback
- `executor.rs` - Translates tool calls to CLI commands
- `tools.rs` - Defines available tools (suggest_search, suggest_list, etc.)

**Instruction design:**
- Keep prompts short and positive ("use X for Y")
- Avoid negative rules ("don't do X") - they're less reliable
- Simple examples work better than complex decision trees

## Gateway Architecture

The communication gateway provides human-in-the-loop approval for AI-initiated messages.

```
AI Agent ──► HTTP API ──► Queue (SQLite) ──► TUI Review ──► Send
                │                               │
                └── poll status ◄───────────────┘
```

**Key components:**
- API key authentication (SHA-256 hashed)
- Message queue with priority ordering
- Local-only approve/deny endpoints (127.0.0.1 check)
- Sends via Gmail API (email) or AppleScript (SMS/iMessage)

**Endpoints:**
- `POST /gateway/send` - Queue message (requires API key)
- `GET /gateway/actions/{id}` - Poll status (requires API key)
- `GET /gateway/queue` - List pending (local only)
- `POST /gateway/queue/{id}/approve` - Approve + send (local only)

## Dependencies

| Crate | Purpose |
|-------|---------|
| clap | CLI parsing |
| rusqlite | SQLite (bundled) |
| uuid | UUIDs |
| chrono | DateTime |
| serde/serde_json | Serialization |
| anyhow/thiserror | Errors |
| dirs | Config paths |
| crossterm | Terminal UI |
| inquire | Interactive prompts |
| reqwest | HTTP client (Gmail API) |
| base64 | Email encoding |
| sha2/hmac/hex | API key hashing |
| ctrlc | Signal handling |
| objc2-contacts | macOS Contacts sync |
