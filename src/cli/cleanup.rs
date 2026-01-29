//! Cleanup mode for bulk deletion of incomplete contacts

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    style::{Attribute, SetAttribute},
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use crate::cli::list::ContactListRow;
use crate::cli::show::run_show;
use crate::cli::ui::{clear_screen, visible_lines};
use crate::db::Database;
use crate::models::Person;
#[cfg(target_os = "macos")]
use crate::cli::sync::{delete_from_macos_contacts_batch, get_apple_id};

const MAX_RETRIES: u32 = 10;
const INITIAL_RETRY_MS: u64 = 2000;  // Start at 2 seconds
const MAX_RETRY_MS: u64 = 30000;     // Cap at 30 seconds

/// RAII guard for raw mode
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Truncate string safely by characters, not bytes
fn truncate_chars(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Run cleanup mode - show incomplete contacts for bulk deletion
pub fn run_cleanup(db: &Database) -> Result<()> {
    let mut cursor: usize = 0;
    let mut scroll: usize = 0;
    // Store full Person data, not just IDs - prevents issues if contact changes between mark and delete
    let mut marked_for_deletion: HashMap<Uuid, Person> = HashMap::new();

    loop {
        clear_screen()?;

        // Fetch incomplete contacts, excluding already-marked ones
        let all_contacts = db.find_persons_missing_both(u32::MAX)?;
        let contacts: Vec<&Person> = all_contacts
            .iter()
            .filter(|p| !marked_for_deletion.contains_key(&p.id))
            .collect();
        let total = contacts.len();

        if total == 0 {
            if marked_for_deletion.is_empty() {
                println!("No incomplete contacts found.");
                println!("\nAll contacts have either email or phone.");
                return Ok(());
            } else {
                // All contacts marked, proceed to deletion
                break;
            }
        }

        // Build display rows
        let rows: Vec<ContactListRow> = contacts
            .iter()
            .map(|p| ContactListRow {
                id: p.id,
                display_name: p.display_name.clone().unwrap_or_else(|| "(unnamed)".into()),
                title_and_org: None,
                primary_email: None,
                primary_phone: None,
                location: None,
            })
            .collect();

        let visible = visible_lines();

        // Clamp cursor to valid range
        if cursor >= total {
            cursor = total.saturating_sub(1);
        }

        // Clamp scroll to valid range
        let max_scroll = total.saturating_sub(visible);
        if scroll > max_scroll {
            scroll = max_scroll;
        }

        // Adjust scroll to keep cursor visible
        if cursor < scroll {
            scroll = cursor;
        } else if cursor >= scroll + visible {
            scroll = cursor.saturating_sub(visible) + 1;
        }

        // Header
        println!("Incomplete Contacts (missing email & phone)\n");
        println!("{:<40}  NOTES", "NAME");

        // Display visible rows
        let end = (scroll + visible).min(total);
        for i in scroll..end {
            let contact = contacts[i];
            let row = &rows[i];
            let notes_preview = contact
                .notes
                .as_ref()
                .map(|n| {
                    let first_line = n.lines().next().unwrap_or("");
                    truncate_chars(first_line, 30)
                })
                .unwrap_or_default();

            let is_selected = i == cursor;

            if is_selected {
                let mut stdout = io::stdout();
                let _ = stdout.execute(SetAttribute(Attribute::Reverse));
                print!("{:<40}  {}", truncate_chars(&row.display_name, 40), notes_preview);
                let _ = stdout.execute(SetAttribute(Attribute::Reset));
                println!();
            } else {
                println!("{:<40}  {}", truncate_chars(&row.display_name, 40), notes_preview);
            }
        }

        // Status bar
        let queued = marked_for_deletion.len();
        if queued > 0 {
            println!(
                "\n{}/{}  queued: {}  [d] mark [D] page [u] undo [enter] view [esc] delete",
                cursor + 1, total, queued
            );
        } else {
            println!(
                "\n{}/{}  [↑↓] move [d] mark [D] mark page [enter] view [esc] back",
                cursor + 1, total
            );
        }

        // Read key
        let code = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                if cursor + 1 < total {
                    cursor += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::Char('d') => {
                // Mark current contact for deletion (instant)
                if cursor < contacts.len() {
                    let person = contacts[cursor];
                    marked_for_deletion.insert(person.id, person.clone());
                }
            }
            KeyCode::Char('D') => {
                // Mark ALL visible contacts on current page
                let start = scroll;
                let end_idx = (scroll + visible).min(total);
                for person in contacts.iter().take(end_idx).skip(start) {
                    marked_for_deletion.insert(person.id, (*person).clone());
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                // Undo - clear all marked deletions
                marked_for_deletion.clear();
            }
            KeyCode::Enter => {
                if cursor < rows.len() {
                    let contact_id = rows[cursor].id;
                    if run_show(db, &contact_id.to_string())? {
                        break;
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                break;
            }
            _ => {}
        }
    }

    // Process deletions
    if marked_for_deletion.is_empty() {
        return Ok(());
    }

    clear_screen()?;
    let total_to_delete = marked_for_deletion.len();
    let persons: Vec<Person> = marked_for_deletion.into_values().collect();

    // Phase 1: Delete from macOS Contacts (batch)
    #[cfg(target_os = "macos")]
    let macos_failed = {
        print!("Phase 1/2: Removing from macOS Contacts... ");
        let _ = io::stdout().flush();

        // Collect all Apple IDs
        let apple_ids: Vec<String> = persons
            .iter()
            .filter_map(get_apple_id)
            .collect();

        let (succeeded, failed) = delete_from_macos_batch_with_retry(&apple_ids);
        println!("{} removed, {} not found", succeeded, failed);
        failed
    };

    #[cfg(not(target_os = "macos"))]
    let macos_failed = 0usize;

    // Phase 2: Batch delete from local database (single transaction)
    println!("Phase 2/2: Removing from local database...");
    let _ = io::stdout().flush();

    let ids: Vec<Uuid> = persons.iter().map(|p| p.id).collect();

    let deleted = match delete_from_db_with_retry(db, &ids) {
        Ok(count) => count,
        Err(e) => {
            eprintln!("\nDatabase error: {}", e);
            0
        }
    };

    // Summary
    println!("\nDeleted: {}", deleted);
    if deleted < total_to_delete {
        println!("Failed (database): {}", total_to_delete - deleted);
    }
    if macos_failed > 0 {
        println!("Failed (macOS Contacts): {}", macos_failed);
    }

    Ok(())
}

/// Calculate retry wait time with exponential backoff, capped at MAX_RETRY_MS
fn calc_retry_wait(attempt: u32) -> u64 {
    let wait = INITIAL_RETRY_MS.saturating_mul(1 << attempt);
    wait.min(MAX_RETRY_MS)
}

/// Check if an error is a retryable database error (locked/busy)
fn is_retryable_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("database is locked")
        || msg.contains("database is busy")
        || msg.contains("unable to open database")
        || msg.contains("disk i/o error")
        || msg.contains("locked")
        || msg.contains("busy")
}

/// Batch delete from macOS Contacts with retry
#[cfg(target_os = "macos")]
fn delete_from_macos_batch_with_retry(apple_ids: &[String]) -> (usize, usize) {
    for attempt in 0..MAX_RETRIES {
        match delete_from_macos_contacts_batch(apple_ids) {
            Ok((succeeded, not_found)) => return (succeeded, not_found),
            Err(e) if is_retryable_error(&e) && attempt < MAX_RETRIES - 1 => {
                let wait_ms = calc_retry_wait(attempt);
                print!("\r  [macOS locked, retry {}/{} in {}s...]            ",
                    attempt + 1, MAX_RETRIES, wait_ms / 1000);
                let _ = io::stdout().flush();
                thread::sleep(Duration::from_millis(wait_ms));
            }
            Err(_) => return (0, apple_ids.len()),
        }
    }
    (0, apple_ids.len())
}

/// Batch delete from database with retry
fn delete_from_db_with_retry(db: &Database, ids: &[Uuid]) -> Result<usize> {
    for attempt in 0..MAX_RETRIES {
        match db.delete_persons_batch(ids) {
            Ok(count) => return Ok(count),
            Err(e) if is_retryable_error(&e) && attempt < MAX_RETRIES - 1 => {
                let wait_ms = calc_retry_wait(attempt);
                print!("\r  [db locked, retry {}/{} in {}s...]               ",
                    attempt + 1, MAX_RETRIES, wait_ms / 1000);
                let _ = io::stdout().flush();
                thread::sleep(Duration::from_millis(wait_ms));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(0)
}
