#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::{run_sync_mac, delete_from_macos_contacts, delete_from_macos_contacts_batch, get_apple_id};

use anyhow::{anyhow, Result};
use crate::db::Database;

/// Execute the sync command
pub fn run_sync(db: &Database, source: &str, dry_run: bool) -> Result<()> {
    match source.to_lowercase().as_str() {
        #[cfg(target_os = "macos")]
        "mac" | "macos" | "apple" => run_sync_mac(db, dry_run),

        #[cfg(not(target_os = "macos"))]
        "mac" | "macos" | "apple" => Err(anyhow!("macOS sync is only available on macOS")),

        _ => Err(anyhow!("Unknown sync source: {}. Available: mac", source)),
    }
}
