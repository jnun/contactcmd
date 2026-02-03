//! Communication Gateway for AI agent message approval.
//!
//! This module provides an HTTP gateway that allows external AI agents
//! to queue messages (SMS, email) requiring human approval before sending.

use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use daemonize::Daemonize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub mod approve;
mod execute;
pub mod filter;
pub mod keys;
mod server;
pub mod types;
pub mod webhook;

pub use server::GatewayServer;

use crate::db::Database;

/// Default port for the gateway server.
const DEFAULT_GATEWAY_PORT: u16 = 9810;

#[derive(Args)]
pub struct GatewayArgs {
    #[command(subcommand)]
    pub command: GatewayCommands,
}

#[derive(Subcommand)]
pub enum GatewayCommands {
    /// Start the gateway server
    Start {
        /// Port to listen on (default: 9810)
        #[arg(short, long, default_value_t = DEFAULT_GATEWAY_PORT)]
        port: u16,

        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the gateway server
    Stop,
    /// Show gateway status
    Status,
    /// Interactive approval interface
    Approve,
    /// Show message history (audit log)
    History {
        /// Filter by status (pending, approved, denied, sent, failed)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by agent name
        #[arg(short, long)]
        agent: Option<String>,

        /// Maximum entries to show (default: 50)
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Manage API keys
    Keys {
        #[command(subcommand)]
        command: KeysCommands,
    },
}

#[derive(Subcommand)]
pub enum KeysCommands {
    /// Generate a new API key
    Add {
        /// Name for the key (e.g., "N8N Agent", "OpenClaw")
        name: String,
    },
    /// List all API keys
    List,
    /// Revoke an API key
    Revoke {
        /// Key ID or prefix to revoke
        id: String,
    },
    /// Manage recipient allowlist for an API key
    Allowlist {
        #[command(subcommand)]
        command: AllowlistCommands,
    },
    /// Set or remove webhook URL for status notifications
    Webhook {
        /// Key ID or prefix (e.g., "abc123" or "gw_abc")
        key_id: String,
        /// Webhook URL to receive status notifications (omit to show current, use --remove to clear)
        url: Option<String>,
        /// Remove the webhook URL
        #[arg(long)]
        remove: bool,
    },
}

#[derive(Subcommand)]
pub enum AllowlistCommands {
    /// Add a recipient pattern to the allowlist
    #[command(alias = "add")]
    Set {
        /// Key ID or prefix (e.g., "abc123" or "gw_abc")
        key_id: String,
        /// Pattern to allow (e.g., "john@example.com", "*@acme.com", "+15551234567")
        pattern: String,
    },
    /// List allowlist patterns for a key
    List {
        /// Key ID or prefix (e.g., "abc123" or "gw_abc")
        key_id: String,
    },
    /// Remove a pattern from the allowlist
    Remove {
        /// Key ID or prefix (e.g., "abc123" or "gw_abc")
        key_id: String,
        /// Pattern to remove
        pattern: String,
    },
}

/// Run the gateway command.
pub fn run_gateway(db: &Database, args: GatewayArgs) -> Result<()> {
    match args.command {
        GatewayCommands::Start { port, foreground } => start_gateway(db, port, foreground),
        GatewayCommands::Stop => stop_gateway(),
        GatewayCommands::Status => show_status(db),
        GatewayCommands::Approve => approve::run_approve(db).map(|_| ()),
        GatewayCommands::History {
            status,
            agent,
            limit,
        } => show_history(db, status, agent, limit),
        GatewayCommands::Keys { command } => match command {
            KeysCommands::Add { name } => add_key(db, &name),
            KeysCommands::List => list_keys(db),
            KeysCommands::Revoke { id } => revoke_key(db, &id),
            KeysCommands::Allowlist { command } => match command {
                AllowlistCommands::Set { key_id, pattern } => allowlist_add(db, &key_id, &pattern),
                AllowlistCommands::List { key_id } => allowlist_list(db, &key_id),
                AllowlistCommands::Remove { key_id, pattern } => {
                    allowlist_remove(db, &key_id, &pattern)
                }
            },
            KeysCommands::Webhook { key_id, url, remove } => {
                webhook_manage(db, &key_id, url.as_deref(), remove)
            }
        },
    }
}

/// Start the gateway server.
fn start_gateway(db: &Database, port: u16, foreground: bool) -> Result<()> {
    // Check if already running
    if let Some(pid) = read_pid_file()? {
        if is_process_running(pid) {
            return Err(anyhow!("Gateway already running (PID {})", pid));
        }
        // Stale PID file, remove it
        remove_pid_file()?;
    }

    if foreground {
        // Run in foreground
        write_pid_file(std::process::id())?;

        let server = GatewayServer::new(port, db)?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Set up Ctrl+C handler
        ctrlc_handler(shutdown_clone);

        println!("Starting gateway server on port {}...", port);
        println!("Press Ctrl+C to stop");

        match server.start(shutdown) {
            Ok(()) => {
                remove_pid_file()?;
                println!("Gateway stopped");
            }
            Err(e) => {
                remove_pid_file()?;
                return Err(e);
            }
        }
    } else {
        // Run as background daemon
        let pid_path = pid_file_path()?;
        let log_path = log_file_path()?;

        // Ensure parent directories exist
        if let Some(parent) = pid_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Open/create log file for appending
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        // Print startup message before daemonizing (parent exits after fork)
        println!("Starting gateway daemon on port {}...", port);
        println!("Log file: {}", log_path.display());
        println!("Stop with: contactcmd gateway stop");

        let daemonize = Daemonize::new()
            .pid_file(&pid_path)
            .chown_pid_file(true)
            .working_directory(".")
            .stdout(log_file.try_clone()?)
            .stderr(log_file);

        match daemonize.start() {
            Ok(_) => {
                // We're now in the daemon process
                // Write startup message to log
                if let Ok(mut f) = OpenOptions::new().append(true).open(&log_path) {
                    let _ = writeln!(
                        f,
                        "[{}] Gateway daemon started on port {}",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        port
                    );
                }

                // Start the server
                let server = GatewayServer::new(port, db)?;
                let shutdown = Arc::new(AtomicBool::new(false));
                let shutdown_clone = shutdown.clone();

                // Set up signal handler for graceful shutdown
                ctrlc_handler(shutdown_clone);

                match server.start(shutdown) {
                    Ok(()) => {
                        if let Ok(mut f) = OpenOptions::new().append(true).open(&log_path) {
                            let _ = writeln!(
                                f,
                                "[{}] Gateway daemon stopped",
                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                            );
                        }
                    }
                    Err(e) => {
                        if let Ok(mut f) = OpenOptions::new().append(true).open(&log_path) {
                            let _ = writeln!(
                                f,
                                "[{}] Gateway daemon error: {}",
                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow!("Failed to daemonize: {}", e));
            }
        }
        // Note: Parent process exits after fork, so this is only reached by daemon
    }

    Ok(())
}

/// Stop the gateway server.
fn stop_gateway() -> Result<()> {
    match read_pid_file()? {
        Some(pid) => {
            if is_process_running(pid) {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                println!("Sent stop signal to gateway (PID {})", pid);

                std::thread::sleep(std::time::Duration::from_millis(500));

                if !is_process_running(pid) {
                    remove_pid_file()?;
                    println!("Gateway stopped");
                } else {
                    println!("Gateway still running, may take a moment to stop");
                }
            } else {
                remove_pid_file()?;
                println!("Gateway was not running (stale PID file removed)");
            }
        }
        None => {
            println!("Gateway is not running");
        }
    }
    Ok(())
}

/// Show gateway status.
fn show_status(db: &Database) -> Result<()> {
    println!("Gateway Status");
    println!("──────────────");

    // Check if running
    match read_pid_file()? {
        Some(pid) if is_process_running(pid) => {
            println!("Status:       Running (PID {})", pid);
            if let Ok(log_path) = log_file_path() {
                if log_path.exists() {
                    println!("Log file:     {}", log_path.display());
                }
            }
        }
        Some(_) => {
            println!("Status:       Stopped (stale PID file)");
        }
        None => {
            println!("Status:       Stopped");
        }
    }

    // Show pending count
    let pending = db.count_pending_queue()?;
    println!("Pending:      {} message(s)", pending);

    // Show key count
    let keys = db.list_api_keys()?;
    let active_keys = keys.iter().filter(|k| k.revoked_at.is_none()).count();
    println!("API Keys:     {} active", active_keys);

    Ok(())
}

/// Show message history (audit log).
fn show_history(
    db: &Database,
    status_filter: Option<String>,
    agent_filter: Option<String>,
    limit: usize,
) -> Result<()> {
    let entries = db.list_queue_history(
        status_filter.as_deref(),
        agent_filter.as_deref(),
        limit,
    )?;

    if entries.is_empty() {
        println!("No messages found.");
        if status_filter.is_some() || agent_filter.is_some() {
            println!("Try removing filters to see all history.");
        }
        return Ok(());
    }

    println!("Gateway Message History");
    println!("───────────────────────");
    if let Some(ref status) = status_filter {
        println!("Filter: status={}", status);
    }
    if let Some(ref agent) = agent_filter {
        println!("Filter: agent={}", agent);
    }
    println!();

    // Header
    println!(
        "{:<19}  {:<8}  {:<10}  {:<20}  {:<8}  {}",
        "TIMESTAMP", "STATUS", "AGENT", "RECIPIENT", "CHANNEL", "PREVIEW"
    );
    println!(
        "{:<19}  {:<8}  {:<10}  {:<20}  {:<8}  {}",
        "─".repeat(19),
        "─".repeat(8),
        "─".repeat(10),
        "─".repeat(20),
        "─".repeat(8),
        "─".repeat(30)
    );

    for (entry, agent_name) in &entries {
        let timestamp = entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string();

        let status_display = match entry.status.as_str() {
            "sent" => "sent",
            "denied" => "DENIED",
            "failed" => "FAILED",
            "pending" => "pending",
            "flagged" => "FLAGGED",
            "approved" => "approved",
            _ => &entry.status,
        };

        let agent_short = if agent_name.len() > 10 {
            format!("{}...", &agent_name[..7])
        } else {
            agent_name.clone()
        };

        let recipient = entry
            .recipient_name
            .as_ref()
            .map(|n| {
                if n.len() > 18 {
                    format!("{}...", &n[..15])
                } else {
                    n.clone()
                }
            })
            .unwrap_or_else(|| {
                if entry.recipient_address.len() > 18 {
                    format!("{}...", &entry.recipient_address[..15])
                } else {
                    entry.recipient_address.clone()
                }
            });

        // Preview: subject for email, body preview otherwise
        let preview = if entry.channel == "email" {
            entry
                .subject
                .as_ref()
                .map(|s| {
                    if s.len() > 30 {
                        format!("{}...", &s[..27])
                    } else {
                        s.clone()
                    }
                })
                .unwrap_or_else(|| "(no subject)".to_string())
        } else {
            let body_first_line = entry.body.lines().next().unwrap_or("");
            if body_first_line.len() > 30 {
                format!("{}...", &body_first_line[..27])
            } else {
                body_first_line.to_string()
            }
        };

        println!(
            "{:<19}  {:<8}  {:<10}  {:<20}  {:<8}  {}",
            timestamp, status_display, agent_short, recipient, entry.channel, preview
        );

        // Show error message for failed entries
        if entry.status == "failed" {
            if let Some(ref err) = entry.error_message {
                let err_preview = if err.len() > 60 {
                    format!("{}...", &err[..57])
                } else {
                    err.clone()
                };
                println!("  └─ Error: {}", err_preview);
            }
        }
    }

    println!();
    println!("Showing {} of {} entries", entries.len(), entries.len());
    if entries.len() == limit {
        println!("Use --limit to show more entries");
    }

    Ok(())
}

/// Add a new API key.
fn add_key(db: &Database, name: &str) -> Result<()> {
    let (full_key, key_hash, key_prefix) = keys::generate_api_key();
    let id = uuid::Uuid::new_v4().to_string();

    db.insert_api_key(&id, name, &key_hash, &key_prefix)?;

    println!("Generated new API key for '{}':\n", name);
    println!("  {}", full_key);
    println!();
    println!("Store this key securely - it cannot be recovered.");
    println!("Key ID: {}", &id[..8]);

    Ok(())
}

/// List all API keys.
fn list_keys(db: &Database) -> Result<()> {
    let keys = db.list_api_keys()?;

    if keys.is_empty() {
        println!("No API keys configured.");
        println!("Use 'contactcmd gateway keys add <name>' to create one.");
        return Ok(());
    }

    println!("API Keys:");
    println!("─────────");

    for key in keys {
        let status = if key.revoked_at.is_some() {
            "REVOKED"
        } else {
            "active"
        };

        let last_used = key
            .last_used_at
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".to_string());

        println!(
            "  {} | {} | {} | last used: {}",
            &key.id[..8],
            key.key_prefix,
            status,
            last_used
        );
        println!("    Name: {}", key.name);
        if let Some(ref webhook_url) = key.webhook_url {
            // Truncate long URLs for display
            let url_display = if webhook_url.len() > 50 {
                format!("{}...", &webhook_url[..47])
            } else {
                webhook_url.clone()
            };
            println!("    Webhook: {}", url_display);
        }
    }

    Ok(())
}

/// Revoke an API key.
fn revoke_key(db: &Database, id_or_prefix: &str) -> Result<()> {
    let keys = db.list_api_keys()?;

    // Find matching key by ID prefix or key prefix
    let matching: Vec<_> = keys
        .iter()
        .filter(|k| k.id.starts_with(id_or_prefix) || k.key_prefix.starts_with(id_or_prefix))
        .collect();

    match matching.len() {
        0 => {
            println!("No key found matching '{}'", id_or_prefix);
        }
        1 => {
            let key = matching[0];
            if key.revoked_at.is_some() {
                println!("Key '{}' is already revoked", key.name);
            } else {
                db.revoke_api_key(&key.id)?;
                println!("Revoked key '{}' ({})", key.name, key.key_prefix);
            }
        }
        _ => {
            println!("Multiple keys match '{}'. Be more specific:", id_or_prefix);
            for key in matching {
                println!("  {} | {}", &key.id[..8], key.name);
            }
        }
    }

    Ok(())
}

// ========== Allowlist Management ==========

/// Find a single API key by ID or prefix, with helpful error messages.
fn find_key_by_prefix<'a>(
    keys: &'a [crate::db::gateway::ApiKey],
    id_or_prefix: &str,
) -> Result<&'a crate::db::gateway::ApiKey> {
    let matching: Vec<_> = keys
        .iter()
        .filter(|k| k.id.starts_with(id_or_prefix) || k.key_prefix.starts_with(id_or_prefix))
        .collect();

    match matching.len() {
        0 => Err(anyhow!("No key found matching '{}'", id_or_prefix)),
        1 => Ok(matching[0]),
        _ => {
            let mut msg = format!("Multiple keys match '{}'. Be more specific:\n", id_or_prefix);
            for key in matching {
                msg.push_str(&format!("  {} | {}\n", &key.id[..8], key.name));
            }
            Err(anyhow!("{}", msg.trim_end()))
        }
    }
}

/// Add a pattern to an API key's allowlist.
fn allowlist_add(db: &Database, id_or_prefix: &str, pattern: &str) -> Result<()> {
    let keys = db.list_api_keys()?;
    let key = find_key_by_prefix(&keys, id_or_prefix)?;

    let entry_id = uuid::Uuid::new_v4().to_string();
    let inserted = db.insert_allowlist_entry(&entry_id, &key.id, pattern)?;

    if inserted {
        println!("Added '{}' to allowlist for '{}' ({})", pattern, key.name, key.key_prefix);
    } else {
        println!("Pattern '{}' already in allowlist for '{}' ({})", pattern, key.name, key.key_prefix);
    }

    // Show current allowlist
    let entries = db.list_allowlist_entries(&key.id)?;
    if !entries.is_empty() {
        println!("\nCurrent allowlist ({} pattern{}):", entries.len(), if entries.len() == 1 { "" } else { "s" });
        for entry in entries {
            println!("  - {}", entry.recipient_pattern);
        }
    }

    Ok(())
}

/// List patterns in an API key's allowlist.
fn allowlist_list(db: &Database, id_or_prefix: &str) -> Result<()> {
    let keys = db.list_api_keys()?;
    let key = find_key_by_prefix(&keys, id_or_prefix)?;

    let entries = db.list_allowlist_entries(&key.id)?;

    println!("Allowlist for '{}' ({})", key.name, key.key_prefix);
    println!("─────────────────────────────────");

    if entries.is_empty() {
        println!("No patterns configured (unrestricted)");
        println!("\nAdd patterns with:");
        println!("  contactcmd gateway keys allowlist add {} <pattern>", &key.id[..8]);
        println!("\nExamples:");
        println!("  contactcmd gateway keys allowlist add {} 'john@example.com'", &key.id[..8]);
        println!("  contactcmd gateway keys allowlist add {} '*@acme.com'", &key.id[..8]);
        println!("  contactcmd gateway keys allowlist add {} '+15551234567'", &key.id[..8]);
    } else {
        println!("{} pattern{}:", entries.len(), if entries.len() == 1 { "" } else { "s" });
        for entry in entries {
            let added = entry.created_at.format("%Y-%m-%d").to_string();
            println!("  {} (added {})", entry.recipient_pattern, added);
        }
    }

    Ok(())
}

/// Remove a pattern from an API key's allowlist.
fn allowlist_remove(db: &Database, id_or_prefix: &str, pattern: &str) -> Result<()> {
    let keys = db.list_api_keys()?;
    let key = find_key_by_prefix(&keys, id_or_prefix)?;

    let deleted = db.delete_allowlist_entry(&key.id, pattern)?;

    if deleted {
        println!("Removed '{}' from allowlist for '{}' ({})", pattern, key.name, key.key_prefix);
    } else {
        println!("Pattern '{}' not found in allowlist for '{}' ({})", pattern, key.name, key.key_prefix);
    }

    // Show remaining allowlist
    let entries = db.list_allowlist_entries(&key.id)?;
    if entries.is_empty() {
        println!("\nAllowlist is now empty (key is unrestricted)");
    } else {
        println!("\nRemaining allowlist ({} pattern{}):", entries.len(), if entries.len() == 1 { "" } else { "s" });
        for entry in entries {
            println!("  - {}", entry.recipient_pattern);
        }
    }

    Ok(())
}

// ========== Webhook Management ==========

/// Manage webhook URL for an API key.
fn webhook_manage(db: &Database, id_or_prefix: &str, url: Option<&str>, remove: bool) -> Result<()> {
    let keys = db.list_api_keys()?;
    let key = find_key_by_prefix(&keys, id_or_prefix)?;

    if remove {
        // Remove webhook URL
        db.set_api_key_webhook(&key.id, None)?;
        println!("Removed webhook URL for '{}' ({})", key.name, key.key_prefix);
    } else if let Some(webhook_url) = url {
        // Validate URL format
        if !webhook_url.starts_with("http://") && !webhook_url.starts_with("https://") {
            return Err(anyhow!("Invalid webhook URL: must start with http:// or https://"));
        }

        // Set webhook URL
        db.set_api_key_webhook(&key.id, Some(webhook_url))?;
        println!("Set webhook URL for '{}' ({}):", key.name, key.key_prefix);
        println!("  {}", webhook_url);
        println!();
        println!("Webhook will receive POST requests when messages are:");
        println!("  - sent (after human approval and successful delivery)");
        println!("  - denied (after human rejection)");
        println!("  - failed (after delivery error)");
    } else {
        // Show current webhook URL
        println!("Webhook for '{}' ({})", key.name, key.key_prefix);
        println!("─────────────────────────────────");
        match &key.webhook_url {
            Some(url) => {
                println!("URL: {}", url);
                println!();
                println!("To change: contactcmd gateway keys webhook {} <new-url>", &key.id[..8]);
                println!("To remove: contactcmd gateway keys webhook {} --remove", &key.id[..8]);
            }
            None => {
                println!("No webhook configured");
                println!();
                println!("Set with: contactcmd gateway keys webhook {} <url>", &key.id[..8]);
                println!();
                println!("Example:");
                println!("  contactcmd gateway keys webhook {} 'https://example.com/webhook'", &key.id[..8]);
            }
        }
    }

    Ok(())
}

// ========== PID File Management ==========

fn pid_file_path() -> Result<PathBuf> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
    Ok(config_dir.join("contactcmd").join("gateway.pid"))
}

fn log_file_path() -> Result<PathBuf> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
    Ok(config_dir.join("contactcmd").join("gateway.log"))
}

fn write_pid_file(pid: u32) -> Result<()> {
    let path = pid_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, pid.to_string())?;
    Ok(())
}

fn read_pid_file() -> Result<Option<u32>> {
    let path = pid_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    match content.trim().parse() {
        Ok(pid) => Ok(Some(pid)),
        Err(_) => Ok(None),
    }
}

fn remove_pid_file() -> Result<()> {
    let path = pid_file_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn ctrlc_handler(shutdown: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down...");
        shutdown.store(true, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_path() {
        let path = pid_file_path().unwrap();
        assert!(path.to_string_lossy().contains("contactcmd"));
        assert!(path.to_string_lossy().contains("gateway.pid"));
    }
}
