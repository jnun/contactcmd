# Development Setup

## Prerequisites

- Rust 1.75+ (`rustup update stable`)
- macOS 11+ (for Contacts sync)

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all 12 tests
cargo test db::persons:: # Run specific module
```

## Database

- Production: `~/.config/contactcmd/contacts.db`
- Tests: Use `Database::open_memory()` for isolation

## Adding a Model

1. Create `src/models/newmodel.rs` with struct + `new()` method
2. Add `mod newmodel; pub use newmodel::*;` to `src/models/mod.rs`
3. Add CRUD in `src/db/` if needed
4. Add tests

## Adding a Command

1. Add `Args` struct in `src/cli/mod.rs`
2. Add variant to `Commands` enum
3. Handle in `src/main.rs` match

## Checks

```bash
cargo fmt && cargo clippy && cargo test
```

## AI Chat Setup

The chat interface requires an OpenAI-compatible API endpoint.

```bash
# Configure via setup command
contactcmd setup

# Or set directly in database settings:
# - ai.provider = "remote"
# - ai.api_key = "sk-..."
# - ai.api_url = "https://api.openai.com/v1" (or compatible)
# - ai.model = "gpt-4" (or similar)
```

**Editing AI instructions:**

The AI prompt lives in `src/cli/ai/instructions.md` and is compiled into the binary via `include_str!`. Changes require recompilation.

Tips for editing:
- Keep it short - long prompts confuse the AI
- Use positive language ("use X for Y") not negative ("don't do X")
- Simple examples work better than complex rules

## Debugging

```bash
RUST_LOG=debug cargo run -- list
RUST_BACKTRACE=1 cargo run -- list
```
