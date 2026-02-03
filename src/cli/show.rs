use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use inquire::Text;
use std::io::{self, Write};
use uuid::Uuid;

use crate::db::Database;
use crate::models::{ContactDetail, Person};
use super::display::print_full_contact_with_tasks;
use super::list::{handle_full_edit, handle_notes};
use super::display::format_message_date;
use super::email::{compose_and_send_email, show_email_error, EmailSendResult};
use super::messages::{get_last_message_for_handles, get_messages_for_handles, detect_service_for_phone, DetectedService};
use super::task::run_tasks_for_contact;
use super::ui::{clear_screen, confirm, minimal_render_config, select_contact, task_action_label, visible_lines, RawModeGuard, StatusBar};
#[cfg(target_os = "macos")]
use super::sync::{delete_from_macos_contacts, get_apple_id};

/// Execute the show command
/// Returns Ok(true) if user wants to quit the app
pub fn run_show(db: &Database, identifier: &str) -> Result<bool> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Identifier cannot be empty."));
    }

    // Try parsing as UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return show_by_uuid(db, uuid, identifier);
    }

    // Not a UUID, search by name (no limit for interactive mode)
    let words: Vec<&str> = identifier.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, u32::MAX)?;

    match results.len() {
        0 => {
            println!("No matches.");
            Ok(false)
        }
        1 => {
            show_person_detail(db, &results[0])
        }
        _ => {
            run_selection_menu(db, &results, identifier)
        }
    }
}

fn show_by_uuid(db: &Database, uuid: Uuid, identifier: &str) -> Result<bool> {
    match db.get_contact_detail(uuid)? {
        Some(detail) => {
            interactive_display(db, &detail)
        }
        None => {
            println!("No contact found with ID: {}", identifier);
            Ok(false)
        }
    }
}

/// Print a contact with their last message and pending tasks preview
fn print_contact_with_message_and_tasks(detail: &ContactDetail, pending_tasks: &[crate::models::Task]) {
    // Extract phone numbers and emails for message lookup
    let phones: Vec<String> = detail.phones.iter()
        .map(|p| p.phone_number.clone())
        .collect();
    let emails: Vec<String> = detail.emails.iter()
        .map(|e| e.email_address.clone())
        .collect();

    // Try to get the last message (gracefully handle errors)
    let last_message = get_last_message_for_handles(&phones, &emails).ok().flatten();

    print_full_contact_with_tasks(detail, last_message.as_ref(), pending_tasks);
}

/// Interactive display with edit/delete/notes actions
/// Returns Ok(true) if user wants to quit the app
fn interactive_display(db: &Database, detail: &ContactDetail) -> Result<bool> {
    let mut quit_app = false;

    loop {
        clear_screen()?;

        // Fetch pending tasks for preview (limit to 3 for efficiency)
        let pending_tasks = db.get_pending_tasks_for_person(detail.person.id, 3)?;
        print_contact_with_message_and_tasks(detail, &pending_tasks);

        // Use the count for the status bar label
        let pending_count = pending_tasks.len() as u32;

        let status = StatusBar::new()
            .action("e", "dit")
            .action("n", "ote")
            .action("m", "sg")
            .action("t", &task_action_label(pending_count))
            .action("d", "el")
            .action("q", "/esc")
            .action("Q", "uit")
            .render();
        print!("\n{}: ", status);
        io::stdout().flush()?;

        // Use raw mode for immediate single-key response
        let action = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match action {
            KeyCode::Char('e') | KeyCode::Char('E') => {
                println!();
                handle_full_edit(db, detail)?;
                // Refresh display after edit
                if let Some(updated) = db.get_contact_detail(detail.person.id)? {
                    return interactive_display(db, &updated);
                }
                break;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                println!();
                handle_notes(db, detail)?;
                // Refresh display after edit
                if let Some(updated) = db.get_contact_detail(detail.person.id)? {
                    return interactive_display(db, &updated);
                }
                break;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                println!();
                if show_messages_screen(db, detail)? {
                    clear_screen()?;
                    quit_app = true;
                    break; // Quit requested from messages screen
                }
                return interactive_display(db, detail);
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                println!();
                let person_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                if run_tasks_for_contact(db, detail.person.id, person_name)? {
                    clear_screen()?;
                    quit_app = true;
                    break; // Quit requested from tasks screen
                }
                return interactive_display(db, detail);
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
                println!();

                // Confirm before delete
                if !confirm(&format!("Delete \"{}\"?", display_name))? {
                    return interactive_display(db, detail);
                }

                // Delete from macOS Contacts if synced from there
                #[cfg(target_os = "macos")]
                if let Some(apple_id) = get_apple_id(&detail.person) {
                    if let Err(e) = delete_from_macos_contacts(&apple_id) {
                        eprintln!("Warning: Could not delete from macOS Contacts: {}", e);
                    }
                }

                if db.delete_person(detail.person.id)? {
                    clear_screen()?;
                    println!("Deleted: {}", display_name);
                }
                break;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Back to menu
                clear_screen()?;
                break;
            }
            KeyCode::Char('Q') => {
                // Quit app entirely
                clear_screen()?;
                quit_app = true;
                break;
            }
            _ => {
                // Unknown input, redisplay
            }
        }
    }

    Ok(quit_app)
}

/// Show a scrollable messages screen for a contact with selection support
/// Returns true if user wants to quit the app entirely
pub fn show_messages_screen(db: &Database, detail: &ContactDetail) -> Result<bool> {
    // Collect both phone numbers and email addresses as potential message handles
    let phones: Vec<String> = detail.phones.iter()
        .map(|p| p.phone_number.clone())
        .collect();
    let emails: Vec<String> = detail.emails.iter()
        .map(|e| e.email_address.clone())
        .collect();

    let messages = get_messages_for_handles(&phones, &emails, 50)?;
    let display_name = detail.person.display_name.as_deref().unwrap_or("(unnamed)");
    let can_text = get_send_address(detail).is_some();
    let has_email = !detail.emails.is_empty();

    if messages.is_empty() {
        clear_screen()?;
        println!("No messages for this contact.\n");

        // Build action bar based on available options
        let mut status = StatusBar::new();
        if can_text {
            status = status.action("t", "ext");
        }
        if has_email {
            status = status.action("@", "email");
        }
        status = status.action("q", "/esc").action("Q", "uit");
        print!("{}: ", status.render());
        io::stdout().flush()?;

        // Wait for key press
        loop {
            let action = {
                let _guard = RawModeGuard::new()?;
                match event::read()? {
                    Event::Key(KeyEvent { code, .. }) => code,
                    _ => continue,
                }
            };

            match action {
                KeyCode::Char('t') | KeyCode::Char('T') if can_text => {
                    if let Some(addr) = get_send_address(detail) {
                        match compose_and_send(&addr, display_name)? {
                            SendResult::Sent => {
                                return show_messages_screen(db, detail);
                            }
                            SendResult::Cancelled => {
                                return show_messages_screen(db, detail);
                            }
                            SendResult::Error(msg) => {
                                show_send_error(&msg)?;
                                return show_messages_screen(db, detail);
                            }
                        }
                    }
                }
                KeyCode::Char('@') if has_email => {
                    if let Some(email_addr) = select_email_address(detail)? {
                        match compose_and_send_email(db, &email_addr, display_name)? {
                            EmailSendResult::Sent => {
                                return show_messages_screen(db, detail);
                            }
                            EmailSendResult::Cancelled => {
                                return show_messages_screen(db, detail);
                            }
                            EmailSendResult::Error(msg) => {
                                show_email_error(&msg)?;
                                return show_messages_screen(db, detail);
                            }
                        }
                    } else {
                        // Selection cancelled
                        return show_messages_screen(db, detail);
                    }
                }
                KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(false); // Back to contact
                }
                KeyCode::Char('Q') => {
                    return Ok(true); // Quit app entirely
                }
                _ => {}
            }
        }
    }

    let total_msgs = messages.len();
    let mut selected: usize = 0;

    loop {
        clear_screen()?;
        let num_visible = visible_lines(); // Recalculate for resize support

        // Calculate scroll to keep selection visible
        let scroll = if selected < num_visible / 2 {
            0
        } else if selected + num_visible / 2 >= total_msgs {
            total_msgs.saturating_sub(num_visible)
        } else {
            selected.saturating_sub(num_visible / 2)
        };

        println!("Messages: {}\n", display_name);

        let end = std::cmp::min(scroll + num_visible, total_msgs);
        for (i, msg) in messages[scroll..end].iter().enumerate() {
            let idx = scroll + i;
            let marker = if idx == selected { ">" } else { " " };
            let direction = if msg.is_from_me { "→" } else { "←" };
            let date_str = format_message_date(&msg.date);
            let first_line = msg.text.lines().next().unwrap_or("").trim();
            let text = if first_line.chars().count() <= 50 {
                first_line.to_string()
            } else {
                format!("{}…", first_line.chars().take(49).collect::<String>())
            };
            println!("{} {} {} \"{}\"", marker, direction, date_str, text);
        }

        // Footer with messaging options
        let mut status = StatusBar::new()
            .counter(selected + 1, total_msgs)
            .action("↑/↓", " select")
            .action("enter", " view");
        if can_text {
            status = status.action("t", "ext");
        }
        if has_email {
            status = status.action("@", "email");
        }
        status = status.action("q", "/esc").action("Q", "uit");
        print!("\n{}", status.render());
        io::stdout().flush()?;

        // Read key
        let action = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match action {
            KeyCode::Down | KeyCode::Char('j') => {
                if selected + 1 < total_msgs {
                    selected += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                // Enter full message view with navigation
                let mut view_index = selected;
                loop {
                    match show_full_message(&messages, view_index, display_name)? {
                        FullMessageAction::Back => {
                            selected = view_index; // Update selection to current viewed message
                            break;
                        }
                        FullMessageAction::Quit => {
                            return Ok(true); // Propagate quit
                        }
                        FullMessageAction::Previous => {
                            if view_index > 0 {
                                view_index -= 1;
                            }
                        }
                        FullMessageAction::Next => {
                            if view_index + 1 < total_msgs {
                                view_index += 1;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') if can_text => {
                if let Some(addr) = get_send_address(detail) {
                    match compose_and_send(&addr, display_name)? {
                        SendResult::Sent => {
                            // Return to refresh messages (will show the sent message)
                            return show_messages_screen(db, detail);
                        }
                        SendResult::Cancelled => {
                            // Just continue showing messages
                        }
                        SendResult::Error(msg) => {
                            show_send_error(&msg)?;
                        }
                    }
                }
            }
            KeyCode::Char('@') if has_email => {
                if let Some(email_addr) = select_email_address(detail)? {
                    match compose_and_send_email(db, &email_addr, display_name)? {
                        EmailSendResult::Sent => {
                            return show_messages_screen(db, detail);
                        }
                        EmailSendResult::Cancelled => {
                            // Just continue showing messages
                        }
                        EmailSendResult::Error(msg) => {
                            show_email_error(&msg)?;
                        }
                    }
                }
                // If selection cancelled, just continue showing messages
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(false); // Return to contact card
            }
            KeyCode::Char('Q') => {
                return Ok(true); // Quit app entirely
            }
            _ => {}
        }
    }
}

/// Result of viewing a full message
enum FullMessageAction {
    Back,       // Return to message list/contact
    Quit,       // Quit app entirely
    Previous,   // Navigate to previous message
    Next,       // Navigate to next message
}

/// Show a single message in full with navigation support
fn show_full_message(
    messages: &[super::messages::LastMessage],
    index: usize,
    contact_name: &str,
) -> Result<FullMessageAction> {
    let msg = &messages[index];
    let total = messages.len();

    clear_screen()?;

    let direction = if msg.is_from_me { "To" } else { "From" };
    let date_str = format_message_date(&msg.date);

    println!("{}: {}", direction, contact_name);
    println!("{}\n", date_str);
    println!("{}", msg.text);

    let status = StatusBar::new()
        .counter(index + 1, total)
        .action("↑/↓", " prev/next")
        .action("enter", " back")
        .action("q", "/esc")
        .action("Q", "uit")
        .render();
    print!("\n{}", status);
    io::stdout().flush()?;

    // Wait for key press
    let action = {
        let _guard = RawModeGuard::new()?;
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                break match code {
                    KeyCode::Up | KeyCode::Char('k') => FullMessageAction::Previous,
                    KeyCode::Down | KeyCode::Char('j') => FullMessageAction::Next,
                    KeyCode::Char('q') | KeyCode::Esc => FullMessageAction::Back,
                    KeyCode::Char('Q') => FullMessageAction::Quit,
                    KeyCode::Enter => FullMessageAction::Back,
                    _ => continue,
                };
            }
        }
    };

    Ok(action)
}

fn show_person_detail(db: &Database, person: &Person) -> Result<bool> {
    match db.get_contact_detail(person.id)? {
        Some(detail) => interactive_display(db, &detail),
        None => {
            // This shouldn't happen - we just found this person via search
            eprintln!("Warning: Could not load details for {}", person.id);
            Ok(false)
        }
    }
}

fn run_selection_menu(db: &Database, results: &[Person], _query: &str) -> Result<bool> {
    if let Some(person) = select_contact(db, results)? {
        return show_person_detail(db, &person);
    }
    Ok(false)
}

// ==================== iMessage Send Functions ====================

/// Result of attempting to send a message
enum SendResult {
    Sent,
    Cancelled,
    Error(String),
}

/// Get the best address to send a message to (phone preferred, email fallback)
fn get_send_address(detail: &ContactDetail) -> Option<String> {
    // Prefer phone number (works for both iMessage and SMS)
    if let Some(phone) = detail.phones.first() {
        return Some(phone.phone_number.clone());
    }
    // Fall back to email (iMessage only)
    if let Some(email) = detail.emails.first() {
        return Some(email.email_address.clone());
    }
    None
}

/// Select which email address to use when a contact has multiple emails.
/// Returns None if cancelled, Some(email_address) if selected.
fn select_email_address(detail: &ContactDetail) -> Result<Option<String>> {
    use super::ui::{minimal_render_config, visible_lines};
    use inquire::Select;

    let emails = &detail.emails;

    if emails.is_empty() {
        return Ok(None);
    }

    // Single email - use it directly
    if emails.len() == 1 {
        return Ok(Some(emails[0].email_address.clone()));
    }

    // Multiple emails - show selection
    clear_screen()?;

    // Build display options: "type: address" or "type: address (primary)"
    let options: Vec<String> = emails
        .iter()
        .map(|e| {
            let type_label = e.email_type.as_str();
            if e.is_primary {
                format!("{}: {} (primary)", type_label, e.email_address)
            } else {
                format!("{}: {}", type_label, e.email_address)
            }
        })
        .collect();

    let result = Select::new("Select email:", options.clone())
        .with_render_config(minimal_render_config())
        .with_page_size(visible_lines())
        .with_vim_mode(true)
        .prompt_skippable()?;

    match result {
        Some(selected) => {
            // Find which email was selected
            let idx = options.iter().position(|o| *o == selected).unwrap_or(0);
            Ok(Some(emails[idx].email_address.clone()))
        }
        None => Ok(None),
    }
}

/// Show compose screen and send message
/// Returns SendResult indicating outcome
fn compose_and_send(recipient: &str, display_name: &str) -> Result<SendResult> {
    clear_screen()?;

    println!("To: {} ({})\n", display_name, recipient);

    let message = Text::new("message:")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let Some(message) = message else {
        return Ok(SendResult::Cancelled);
    };

    if message.trim().is_empty() {
        return Ok(SendResult::Cancelled);
    }

    print!("Sending...");
    io::stdout().flush()?;

    match send_imessage(recipient, &message) {
        Ok(()) => {
            println!(" Sent.");
            std::thread::sleep(std::time::Duration::from_millis(800));
            Ok(SendResult::Sent)
        }
        Err(e) => Ok(SendResult::Error(e.to_string())),
    }
}

/// Send a message via AppleScript (supports both iMessage and SMS)
#[cfg(target_os = "macos")]
fn send_imessage(recipient: &str, message: &str) -> Result<()> {
    use std::process::Command;

    // Escape message for AppleScript
    let escaped_message = message.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_recipient = recipient.replace('\\', "\\\\").replace('"', "\\\"");

    // Determine if this looks like a phone number (for SMS) or email (for iMessage)
    let is_phone = recipient.chars().any(|c| c.is_ascii_digit())
        && !recipient.contains('@');

    // For phone numbers, check chat history to detect the appropriate service.
    // This avoids sending iMessage to Android users (which fails silently) or
    // SMS to iPhone users (green bubbles).
    let script = if is_phone {
        // Check what service was used in previous conversations
        let detected = detect_service_for_phone(recipient).unwrap_or(DetectedService::Unknown);

        match detected {
            DetectedService::Sms => {
                // Known Android user - use SMS directly
                format!(
                    r#"
                    tell application "Messages"
                        set smsService to 1st account whose service type = SMS
                        set targetBuddy to participant "{0}" of smsService
                        send "{1}" to targetBuddy
                    end tell
                    "#,
                    escaped_recipient, escaped_message
                )
            }
            DetectedService::IMessage => {
                // Known iPhone user - use iMessage directly
                format!(
                    r#"
                    tell application "Messages"
                        set imsgService to 1st account whose service type = iMessage
                        set targetBuddy to participant "{0}" of imsgService
                        send "{1}" to targetBuddy
                    end tell
                    "#,
                    escaped_recipient, escaped_message
                )
            }
            DetectedService::Unknown => {
                // No history - try iMessage first, fall back to SMS
                format!(
                    r#"
                    tell application "Messages"
                        -- Try iMessage first (blue bubbles, preferred)
                        try
                            set imsgService to 1st account whose service type = iMessage
                            set targetBuddy to participant "{0}" of imsgService
                            send "{1}" to targetBuddy
                            return "sent"
                        end try

                        -- Fall back to SMS (green bubbles, for Android users)
                        try
                            set smsService to 1st account whose service type = SMS
                            set targetBuddy to participant "{0}" of smsService
                            send "{1}" to targetBuddy
                            return "sent"
                        end try

                        error "Could not find iMessage or SMS service"
                    end tell
                    "#,
                    escaped_recipient, escaped_message
                )
            }
        }
    } else {
        // Email addresses use iMessage only
        format!(
            r#"
            tell application "Messages"
                set imsgService to 1st account whose service type = iMessage
                set targetBuddy to participant "{}" of imsgService
                send "{}" to targetBuddy
            end tell
            "#,
            escaped_recipient, escaped_message
        )
    };

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_lower = stderr.to_lowercase();

        if stderr_lower.contains("not authorized") || stderr_lower.contains("assistive access") {
            anyhow::bail!(
                "Permission required.\n\n\
                Grant access in: System Settings > Privacy & Security > Accessibility\n\
                Add your terminal app (Terminal, iTerm, etc.) to the list."
            );
        }
        if stderr_lower.contains("can't get account") || stderr_lower.contains("no account") {
            if is_phone {
                anyhow::bail!(
                    "No iMessage or SMS service available.\n\n\
                    For iMessage: Open Messages.app and sign in with your Apple ID.\n\
                    For SMS: Connect your iPhone and enable Settings > Messages > Text Message Forwarding."
                );
            } else {
                anyhow::bail!(
                    "Messages.app is not set up.\n\n\
                    Open Messages.app and sign in with your Apple ID first."
                );
            }
        }
        anyhow::bail!("Send failed: {}", stderr.trim());
    }

    Ok(())
}

/// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
fn send_imessage(_recipient: &str, _message: &str) -> Result<()> {
    anyhow::bail!("Sending messages is only available on macOS.")
}

/// Show an error message and wait for keypress
fn show_send_error(message: &str) -> Result<()> {
    clear_screen()?;
    println!("Error: {}\n", message);
    let status = StatusBar::new()
        .action("enter", "/")
        .action("q", " to continue")
        .render();
    print!("{}", status);
    io::stdout().flush()?;

    let _guard = RawModeGuard::new()?;
    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            if matches!(code, KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc) {
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Email, Phone, Address, AddressType, EmailType, PhoneType};

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        // Create contacts
        let contacts = vec![
            ("John", "Smith"),
            ("Jane", "Smith"),
            ("John", "Doe"),
        ];

        for (first, last) in contacts {
            let mut p = Person::new();
            p.name_given = Some(first.to_string());
            p.name_family = Some(last.to_string());
            p.compute_names();
            db.insert_person(&p).unwrap();

            // Add email for first contact
            if first == "John" && last == "Smith" {
                let mut email = Email::new(p.id, "john@example.com".to_string());
                email.email_type = EmailType::Work;
                email.is_primary = true;
                db.insert_email(&email).unwrap();

                let mut phone = Phone::new(p.id, "(555) 123-4567".to_string());
                phone.phone_type = PhoneType::Mobile;
                phone.is_primary = true;
                db.insert_phone(&phone).unwrap();

                let mut addr = Address::new(p.id);
                addr.city = Some("Austin".to_string());
                addr.state = Some("TX".to_string());
                addr.address_type = AddressType::Home;
                addr.is_primary = true;
                db.insert_address(&addr).unwrap();
            }
        }

        db
    }

    #[test]
    fn test_show_by_uuid() {
        let db = setup_test_db();

        // Get the UUID of John Smith
        let results = db.search_persons_multi(&["john", "smith"], false, 1).unwrap();
        assert_eq!(results.len(), 1);

        let uuid = results[0].id;
        let detail = db.get_contact_detail(uuid).unwrap();
        assert!(detail.is_some());

        let detail = detail.unwrap();
        assert_eq!(detail.person.name_given, Some("John".to_string()));
        assert_eq!(detail.emails.len(), 1);
        assert_eq!(detail.phones.len(), 1);
        assert_eq!(detail.addresses.len(), 1);
    }

    #[test]
    fn test_search_single_match() {
        let db = setup_test_db();

        // "John Smith" should match exactly one person
        let results = db.search_persons_multi(&["john", "smith"], false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, Some("John Smith".to_string()));
    }

    #[test]
    fn test_search_multiple_matches() {
        let db = setup_test_db();

        // "Smith" should match two people
        let results = db.search_persons_multi(&["smith"], false, 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_no_match() {
        let db = setup_test_db();

        let results = db.search_persons_multi(&["nonexistent"], false, 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_empty_identifier_error() {
        let db = setup_test_db();
        let result = run_show(&db, "   ");
        assert!(result.is_err());
    }
}
