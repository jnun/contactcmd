//! Interactive approval TUI for the gateway.
//!
//! Displays pending messages in a DOS-style list with keyboard navigation.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Attribute, SetAttribute},
    ExecutableCommand,
};
use std::io::{self, Write};

use super::execute;
use super::webhook;
use crate::cli::ui::{clear_screen, truncate, RawModeGuard, StatusBar};
use crate::db::gateway::QueueEntry;
use crate::db::Database;

/// Run the interactive approval list interface.
pub fn run_approve(db: &Database) -> Result<bool> {
    let mut selected_idx: usize = 0;
    let mut show_detail = false;
    let mut detail_entry: Option<QueueEntry> = None;

    loop {
        let entries = db.list_pending_queue()?;

        // Clamp selection
        if entries.is_empty() {
            selected_idx = 0;
        } else if selected_idx >= entries.len() {
            selected_idx = entries.len().saturating_sub(1);
        }

        clear_screen()?;
        let mut stdout = io::stdout();

        if show_detail {
            if let Some(ref entry) = detail_entry {
                render_detail(&mut stdout, db, entry)?;

                let status = StatusBar::new()
                    .action("a", "pprove")
                    .action("d", "eny")
                    .action("esc", " back")
                    .action("q", "uit")
                    .render();
                println!("{}", status);
                stdout.flush()?;

                let code = read_key()?;
                match code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        let result = approve_entry(db, entry);
                        show_result(&mut stdout, &result)?;
                        show_detail = false;
                        detail_entry = None;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        deny_entry(db, entry)?;
                        show_detail = false;
                        detail_entry = None;
                    }
                    KeyCode::Esc | KeyCode::Backspace => {
                        show_detail = false;
                        detail_entry = None;
                    }
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Char('Q') => return Ok(true),
                    _ => {}
                }
                continue;
            }
        }

        // Header
        println!("GATEWAY QUEUE ({} pending)\n", entries.len());
        print_header();

        if entries.is_empty() {
            println!("  No pending messages.\n");
        } else {
            for (idx, entry) in entries.iter().enumerate() {
                print_row(&mut stdout, db, entry, idx == selected_idx)?;
            }
        }

        println!();
        let status = StatusBar::new()
            .counter(if entries.is_empty() { 0 } else { selected_idx + 1 }, entries.len())
            .action("enter", " view")
            .action("a", "pprove")
            .action("d", "eny")
            .action("↑/↓", "")
            .action("q", "/esc")
            .action("Q", "uit")
            .render();
        println!("{}", status);
        stdout.flush()?;

        let code = read_key()?;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
            KeyCode::Char('Q') => return Ok(true),
            KeyCode::Up | KeyCode::Char('k') => {
                selected_idx = selected_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !entries.is_empty() && selected_idx < entries.len() - 1 {
                    selected_idx += 1;
                }
            }
            KeyCode::Enter => {
                if !entries.is_empty() {
                    detail_entry = Some(entries[selected_idx].clone());
                    show_detail = true;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if !entries.is_empty() {
                    let entry = &entries[selected_idx];
                    let result = approve_entry(db, entry);
                    show_result(&mut stdout, &result)?;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !entries.is_empty() {
                    let entry = &entries[selected_idx];
                    deny_entry(db, entry)?;
                }
            }
            _ => {}
        }
    }
}

fn read_key() -> Result<KeyCode> {
    let _guard = RawModeGuard::new()?;
    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(KeyCode::Esc);
            }
            return Ok(code);
        }
    }
}

fn print_header() {
    let layout = QueueLayout::default();
    println!(
        "{:<ch$}  {:<to$}  {:<subj$}  {:<agent$}  {:<pri$}",
        "CH",
        "TO",
        "SUBJECT/BODY",
        "AGENT",
        "PRI",
        ch = layout.channel_width,
        to = layout.to_width,
        subj = layout.subject_width,
        agent = layout.agent_width,
        pri = layout.priority_width
    );
}

struct QueueLayout {
    channel_width: usize,
    to_width: usize,
    subject_width: usize,
    agent_width: usize,
    priority_width: usize,
}

impl Default for QueueLayout {
    fn default() -> Self {
        let term_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);

        if term_width >= 100 {
            QueueLayout {
                channel_width: 8,
                to_width: 20,
                subject_width: 30,
                agent_width: 15,
                priority_width: 6,
            }
        } else {
            QueueLayout {
                channel_width: 6,
                to_width: 15,
                subject_width: 20,
                agent_width: 10,
                priority_width: 4,
            }
        }
    }
}

fn print_row(stdout: &mut io::Stdout, db: &Database, entry: &QueueEntry, selected: bool) -> Result<()> {
    let layout = QueueLayout::default();

    // Show flag indicator for flagged entries
    let flag_indicator = if entry.status == "flagged" { "!" } else { " " };
    let channel = entry.channel.to_uppercase();
    let to = entry.recipient_name.as_deref().unwrap_or(&entry.recipient_address);
    let subject = entry.subject.as_deref().unwrap_or(&entry.body);
    let agent = get_agent_name(db, &entry.api_key_id);
    let priority = &entry.priority;

    let line = format!(
        "{}{:<ch$}  {:<to$}  {:<subj$}  {:<agent$}  {:<pri$}",
        flag_indicator,
        truncate(&channel, layout.channel_width),
        truncate(to, layout.to_width),
        truncate(subject, layout.subject_width),
        truncate(&agent, layout.agent_width),
        truncate(priority, layout.priority_width),
        ch = layout.channel_width,
        to = layout.to_width,
        subj = layout.subject_width,
        agent = layout.agent_width,
        pri = layout.priority_width
    );

    if selected {
        stdout.execute(SetAttribute(Attribute::Reverse))?;
        print!("{}", line);
        stdout.execute(SetAttribute(Attribute::Reset))?;
        println!();
    } else {
        println!("{}", line);
    }

    Ok(())
}

fn render_detail(stdout: &mut io::Stdout, db: &Database, entry: &QueueEntry) -> Result<()> {
    let agent = get_agent_name(db, &entry.api_key_id);

    println!("MESSAGE DETAIL\n");
    if entry.status == "flagged" {
        println!("!! FLAGGED - Review carefully (matched content filter) !!\n");
    }
    println!("Agent:     {}", agent);
    println!("Channel:   {}", entry.channel.to_uppercase());
    println!("Priority:  {}", entry.priority);
    println!("Status:    {}", entry.status);
    println!("Queued:    {}", entry.created_at.format("%Y-%m-%d %H:%M"));
    println!();
    println!("To:        {}", entry.recipient_address);
    if let Some(ref name) = entry.recipient_name {
        println!("           ({})", name);
    }
    println!();

    if let Some(ref subject) = entry.subject {
        println!("Subject:   {}", subject);
        println!();
    }

    println!("Message:");
    println!("─────────────────────────────────────────────────────────────");
    for line in entry.body.lines().take(15) {
        println!("  {}", line);
    }
    if entry.body.lines().count() > 15 {
        println!("  ...(truncated)");
    }
    println!("─────────────────────────────────────────────────────────────");
    println!();

    if let Some(ref context) = entry.agent_context {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(context) {
            println!("Context:");
            if let Some(obj) = parsed.as_object() {
                for (key, value) in obj {
                    println!("  {}: {}", key, value);
                }
            }
            println!();
        }
    }

    stdout.flush()?;
    Ok(())
}

fn approve_entry(db: &Database, entry: &QueueEntry) -> ApproveResult {
    if let Err(e) = db.update_queue_status(&entry.id, "approved") {
        return ApproveResult::Error(e.to_string());
    }

    match execute::execute_send(db, entry) {
        Ok(()) => {
            let _ = db.mark_queue_sent(&entry.id);
            let sent_at = chrono::Utc::now().to_rfc3339();
            // Send webhook notification (non-blocking for errors)
            let _ = webhook::notify_status_change(
                db,
                &entry.api_key_id,
                &entry.id,
                "sent",
                &entry.recipient_address,
                &entry.channel,
                Some(&sent_at),
                None,
            );
            ApproveResult::Sent
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = db.mark_queue_failed(&entry.id, &error_msg);
            // Send webhook notification (non-blocking for errors)
            let _ = webhook::notify_status_change(
                db,
                &entry.api_key_id,
                &entry.id,
                "failed",
                &entry.recipient_address,
                &entry.channel,
                None,
                Some(&error_msg),
            );
            ApproveResult::Failed(error_msg)
        }
    }
}

fn deny_entry(db: &Database, entry: &QueueEntry) -> Result<()> {
    db.update_queue_status(&entry.id, "denied")?;
    // Send webhook notification (non-blocking for errors)
    let _ = webhook::notify_status_change(
        db,
        &entry.api_key_id,
        &entry.id,
        "denied",
        &entry.recipient_address,
        &entry.channel,
        None,
        None,
    );
    Ok(())
}

enum ApproveResult {
    Sent,
    Failed(String),
    Error(String),
}

fn show_result(stdout: &mut io::Stdout, result: &ApproveResult) -> Result<()> {
    clear_screen()?;
    match result {
        ApproveResult::Sent => {
            println!("Sent.\n");
        }
        ApproveResult::Failed(e) => {
            println!("Send failed: {}\n", e);
        }
        ApproveResult::Error(e) => {
            println!("Error: {}\n", e);
        }
    }
    println!("Press any key to continue...");
    stdout.flush()?;
    let _ = read_key()?;
    Ok(())
}

fn get_agent_name(db: &Database, api_key_id: &str) -> String {
    db.list_api_keys()
        .ok()
        .and_then(|keys| keys.into_iter().find(|k| k.id == api_key_id))
        .map(|k| k.name)
        .unwrap_or_else(|| "Unknown".to_string())
}
