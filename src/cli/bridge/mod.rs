//! Moltbot bridge for iMessage/SMS integration.
//!
//! This module provides an HTTP bridge between contactcmd (Mac) and
//! Moltbot (Docker) for secure message exchange.

use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod client;
mod server;
mod signing;
mod types;

pub use client::BridgeClient;
pub use server::{BridgeEvent, BridgeServer};
pub use signing::{compute_signature, generate_secret, generate_token, verify_signature};
pub use types::*;

use crate::db::Database;

/// Settings keys for bridge configuration
const SETTING_BRIDGE_SECRET: &str = "bridge_shared_secret";
const SETTING_BRIDGE_TOKEN: &str = "bridge_token";
const SETTING_BRIDGE_MOLTBOT_PORT: &str = "bridge_moltbot_port";

/// Default ports
const DEFAULT_BRIDGE_PORT: u16 = 9800;
const DEFAULT_MOLTBOT_PORT: u16 = 9801;

#[derive(Args)]
pub struct BridgeArgs {
    #[command(subcommand)]
    pub command: BridgeCommands,
}

#[derive(Subcommand)]
pub enum BridgeCommands {
    /// Start the bridge server
    Start {
        /// Port to listen on (default: 9800)
        #[arg(short, long, default_value_t = DEFAULT_BRIDGE_PORT)]
        port: u16,

        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the bridge server
    Stop,
    /// Show bridge status
    Status,
    /// Configure bridge settings
    Config {
        /// Set the shared secret
        #[arg(long)]
        secret: Option<String>,

        /// Set the authentication token
        #[arg(long)]
        token: Option<String>,

        /// Set the Moltbot port
        #[arg(long)]
        moltbot_port: Option<u16>,

        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Generate new secret and token
        #[arg(long)]
        generate: bool,
    },
}

/// Run the bridge command.
pub fn run_bridge(db: &Database, args: BridgeArgs) -> Result<()> {
    match args.command {
        BridgeCommands::Start { port, foreground } => start_bridge(db, port, foreground),
        BridgeCommands::Stop => stop_bridge(),
        BridgeCommands::Status => show_status(db),
        BridgeCommands::Config {
            secret,
            token,
            moltbot_port,
            show,
            generate,
        } => configure_bridge(db, secret, token, moltbot_port, show, generate),
    }
}

/// Start the bridge server.
fn start_bridge(db: &Database, port: u16, foreground: bool) -> Result<()> {
    // Check if already running
    if let Some(pid) = read_pid_file()? {
        if is_process_running(pid) {
            return Err(anyhow!("Bridge already running (PID {})", pid));
        }
        // Stale PID file, remove it
        remove_pid_file()?;
    }

    // Get or generate configuration
    let secret = match db.get_setting(SETTING_BRIDGE_SECRET)? {
        Some(s) => s,
        None => {
            let s = generate_secret();
            db.set_setting(SETTING_BRIDGE_SECRET, &s)?;
            println!("Generated new shared secret");
            s
        }
    };

    let token = db.get_setting(SETTING_BRIDGE_TOKEN)?;

    if foreground {
        // Run in foreground
        write_pid_file(std::process::id())?;

        let server = BridgeServer::new(port, secret, token);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Set up Ctrl+C handler
        ctrlc_handler(shutdown_clone);

        println!("Starting bridge server on port {}...", port);
        println!("Press Ctrl+C to stop");

        match server.start(shutdown) {
            Ok(mut rx) => {
                // Process events until shutdown
                while let Some(event) = rx.blocking_recv() {
                    match event {
                        BridgeEvent::OutboundMessage(msg) => {
                            println!(
                                "Received outbound message: {} -> {} via {}",
                                msg.id, msg.recipient, msg.channel
                            );
                            // TODO: Actually send the message via iMessage/SMS
                        }
                        BridgeEvent::Kill { reason } => {
                            println!(
                                "Kill request received: {}",
                                reason.unwrap_or_else(|| "no reason".to_string())
                            );
                            break;
                        }
                        BridgeEvent::Error(e) => {
                            eprintln!("Bridge error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                remove_pid_file()?;
                return Err(e);
            }
        }

        remove_pid_file()?;
        println!("Bridge stopped");
    } else {
        // Daemonize not implemented yet
        println!("Background mode not yet implemented. Use --foreground flag.");
        println!("Example: contactcmd bridge start --foreground");
    }

    Ok(())
}

/// Stop the bridge server.
fn stop_bridge() -> Result<()> {
    match read_pid_file()? {
        Some(pid) => {
            if is_process_running(pid) {
                // Send SIGTERM
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                println!("Sent stop signal to bridge (PID {})", pid);

                // Wait a moment and check if it stopped
                std::thread::sleep(std::time::Duration::from_millis(500));

                if !is_process_running(pid) {
                    remove_pid_file()?;
                    println!("Bridge stopped");
                } else {
                    println!("Bridge still running, may take a moment to stop");
                }
            } else {
                remove_pid_file()?;
                println!("Bridge was not running (stale PID file removed)");
            }
        }
        None => {
            println!("Bridge is not running");
        }
    }
    Ok(())
}

/// Show bridge status.
fn show_status(db: &Database) -> Result<()> {
    println!("Bridge Status");
    println!("─────────────");

    // Check if running
    match read_pid_file()? {
        Some(pid) if is_process_running(pid) => {
            println!("Status:       Running (PID {})", pid);
        }
        Some(_) => {
            println!("Status:       Stopped (stale PID file)");
        }
        None => {
            println!("Status:       Stopped");
        }
    }

    // Show configuration
    let has_secret = db.get_setting(SETTING_BRIDGE_SECRET)?.is_some();
    let has_token = db.get_setting(SETTING_BRIDGE_TOKEN)?.is_some();
    let moltbot_port = db
        .get_setting(SETTING_BRIDGE_MOLTBOT_PORT)?
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_MOLTBOT_PORT);

    println!("Secret:       {}", if has_secret { "configured" } else { "not set" });
    println!("Token:        {}", if has_token { "configured" } else { "not set" });
    println!("Moltbot port: {}", moltbot_port);

    // Try to check Moltbot health
    if let Some(secret) = db.get_setting(SETTING_BRIDGE_SECRET)? {
        let token = db.get_setting(SETTING_BRIDGE_TOKEN)?;
        match BridgeClient::new(moltbot_port, secret, token) {
            Ok(client) => match client.health_check() {
                Ok(health) => {
                    println!("Moltbot:      {} (uptime {}s)", health.status, health.uptime_secs);
                }
                Err(_) => {
                    println!("Moltbot:      not reachable");
                }
            },
            Err(_) => {
                println!("Moltbot:      client error");
            }
        }
    }

    Ok(())
}

/// Configure bridge settings.
fn configure_bridge(
    db: &Database,
    secret: Option<String>,
    token: Option<String>,
    moltbot_port: Option<u16>,
    show: bool,
    generate: bool,
) -> Result<()> {
    // Generate new credentials if requested
    if generate {
        let new_secret = generate_secret();
        let new_token = generate_token();
        db.set_setting(SETTING_BRIDGE_SECRET, &new_secret)?;
        db.set_setting(SETTING_BRIDGE_TOKEN, &new_token)?;
        println!("Generated new credentials:");
        println!("  Secret: {}", new_secret);
        println!("  Token:  {}", new_token);
        return Ok(());
    }

    // Track if any settings were changed
    let mut changed = false;

    // Set individual values
    if let Some(ref s) = secret {
        db.set_setting(SETTING_BRIDGE_SECRET, s)?;
        println!("Secret updated");
        changed = true;
    }

    if let Some(ref t) = token {
        db.set_setting(SETTING_BRIDGE_TOKEN, t)?;
        println!("Token updated");
        changed = true;
    }

    if let Some(p) = moltbot_port {
        db.set_setting(SETTING_BRIDGE_MOLTBOT_PORT, &p.to_string())?;
        println!("Moltbot port set to {}", p);
        changed = true;
    }

    // Show current configuration
    if show || !changed {
        println!("Bridge Configuration");
        println!("────────────────────");

        match db.get_setting(SETTING_BRIDGE_SECRET)? {
            Some(s) => println!("Secret:       {}...", &s[..8.min(s.len())]),
            None => println!("Secret:       (not set)"),
        }

        match db.get_setting(SETTING_BRIDGE_TOKEN)? {
            Some(t) => println!("Token:        {}...", &t[..8.min(t.len())]),
            None => println!("Token:        (not set)"),
        }

        let port = db
            .get_setting(SETTING_BRIDGE_MOLTBOT_PORT)?
            .unwrap_or_else(|| DEFAULT_MOLTBOT_PORT.to_string());
        println!("Moltbot port: {}", port);
    }

    Ok(())
}

// PID file management

fn pid_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not find config directory"))?;
    Ok(config_dir.join("contactcmd").join("bridge.pid"))
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
        // kill(pid, 0) checks if process exists without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
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
        assert!(path.to_string_lossy().contains("bridge.pid"));
    }
}
