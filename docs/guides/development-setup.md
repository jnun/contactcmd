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

## Debugging

```bash
RUST_LOG=debug cargo run -- list
RUST_BACKTRACE=1 cargo run -- list
```
