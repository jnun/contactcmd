//! Email compose and send functionality via Mail.app or Gmail SMTP

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use inquire::Text;
use std::io::{self, Write};
use std::process::Command;

use crate::db::Database;
use super::google_auth::{is_google_auth_configured, get_google_email, refresh_access_token_if_needed};
use super::setup::{get_default_subject, get_email_account, get_email_signature};
use super::ui::{clear_screen, minimal_render_config, multiline_input_email, wait_for_key, RawModeGuard};

/// Result of attempting to send an email
pub enum EmailSendResult {
    Sent,
    Cancelled,
    Error(String),
}

/// Email send method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailMethod {
    Google,
    MailApp,
}

/// Compose and send an email
/// Returns EmailSendResult indicating outcome
pub fn compose_and_send_email(
    db: &Database,
    to_address: &str,
    display_name: &str,
) -> Result<EmailSendResult> {
    // Determine email method and from address
    let (method, from_address) = if is_google_auth_configured(db) {
        let email = get_google_email(db).unwrap_or_default();
        (EmailMethod::Google, email)
    } else if let Some(addr) = get_email_account(db) {
        (EmailMethod::MailApp, addr)
    } else {
        return Ok(EmailSendResult::Error(
            "Email not configured. Use Setup from main menu to configure.".to_string(),
        ));
    };

    clear_screen()?;

    let method_label = match method {
        EmailMethod::Google => "via Gmail",
        EmailMethod::MailApp => "via Mail.app",
    };

    println!("Compose Email ({})\n", method_label);
    println!("  To: {} ({})", display_name, to_address);
    println!("  From: {}", from_address);
    println!();

    // Get subject (with default if configured)
    let default_subject = get_default_subject(db).unwrap_or_default();
    let subject = Text::new("  Subject:")
        .with_render_config(minimal_render_config())
        .with_initial_value(&default_subject)
        .with_help_message("hit enter to continue")
        .prompt_skippable()?;

    let Some(subject) = subject else {
        return Ok(EmailSendResult::Cancelled);
    };

    println!();

    // Get signature for appending
    let signature = get_email_signature(db).unwrap_or_default();
    let sig_content = if signature.is_empty() {
        String::new()
    } else {
        format!("--\n{}", signature)
    };

    // Multi-line body input with Escape support
    let body = match multiline_input_email(&sig_content)? {
        Some(text) => text,
        None => return Ok(EmailSendResult::Cancelled),
    };

    // Confirm send
    clear_screen()?;
    println!("Ready to send:\n");
    println!("To: {} ({})", display_name, to_address);
    println!("From: {}", from_address);
    println!("Subject: {}\n", subject);

    // Show preview of body (first few lines)
    let preview_lines: Vec<&str> = body.lines().take(5).collect();
    for line in &preview_lines {
        println!("  {}", line);
    }
    if body.lines().count() > 5 {
        println!("  ...");
    }

    println!();

    // Show appropriate options based on method
    let prompt = match method {
        EmailMethod::Google => "[s]end [q]uit: ",
        EmailMethod::MailApp => "[o]pen in Mail [q]uit: ",
    };
    print!("{}", prompt);
    io::stdout().flush()?;

    enum SendChoice {
        SendDirect,
        OpenMail,
        Cancel,
    }

    let choice = {
        let _guard = RawModeGuard::new()?;
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('s') | KeyCode::Char('S') if method == EmailMethod::Google => {
                        break SendChoice::SendDirect;
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') if method == EmailMethod::MailApp => {
                        break SendChoice::OpenMail;
                    }
                    KeyCode::Enter => {
                        // Enter sends directly for Google, opens Mail for Mail.app
                        break match method {
                            EmailMethod::Google => SendChoice::SendDirect,
                            EmailMethod::MailApp => SendChoice::OpenMail,
                        };
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        break SendChoice::Cancel;
                    }
                    _ => {}
                }
            }
        }
    };

    match choice {
        SendChoice::Cancel => Ok(EmailSendResult::Cancelled),
        SendChoice::OpenMail => {
            // Use Mail.app (need a from address for it)
            let mail_from = get_email_account(db).unwrap_or(from_address.clone());
            print!("\nOpening in Mail...");
            io::stdout().flush()?;

            match send_email_via_mail_app(&mail_from, to_address, &subject, &body) {
                Ok(()) => {
                    println!(" Ready. Send from Mail.app window.");
                    std::thread::sleep(std::time::Duration::from_millis(1200));
                    Ok(EmailSendResult::Sent)
                }
                Err(e) => Ok(EmailSendResult::Error(e.to_string())),
            }
        }
        SendChoice::SendDirect => {
            print!("\nSending...");
            io::stdout().flush()?;

            match send_email_via_gmail(db, &from_address, to_address, &subject, &body) {
                Ok(()) => {
                    println!(" Sent.");
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    Ok(EmailSendResult::Sent)
                }
                Err(e) => Ok(EmailSendResult::Error(e.to_string())),
            }
        }
    }
}

/// Send email via Gmail API with OAuth2
fn send_email_via_gmail(
    db: &Database,
    from_addr: &str,
    to_addr: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    use base64::Engine;

    // Get fresh access token
    let access_token = refresh_access_token_if_needed(db)?;

    // Build RFC 2822 email message
    let email_content = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
        from_addr, to_addr, subject, body
    );

    // Base64url encode the message (Gmail API requirement)
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email_content.as_bytes());

    // Send via Gmail API
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
        .bearer_auth(&access_token)
        .json(&serde_json::json!({
            "raw": encoded
        }))
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().unwrap_or_default();
        anyhow::bail!("Gmail API error ({}): {}", status, error_body);
    }

    Ok(())
}

/// Send email via Mail.app using AppleScript
/// Creates a draft, queues it for sending, then lets Mail handle delivery
#[cfg(target_os = "macos")]
fn send_email_via_mail_app(
    from_addr: &str,
    to_addr: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    // Escape strings for AppleScript
    let escaped_from = escape_applescript(from_addr);
    let escaped_to = escape_applescript(to_addr);
    let escaped_subject = escape_applescript(subject);
    let escaped_body = escape_applescript(body);

    // This script:
    // 1. Launches Mail if needed and waits for it
    // 2. Creates the message with visible:true so user can verify
    // 3. Does NOT auto-send - user clicks send in Mail.app
    // This avoids automation loops and gives user control
    let script = format!(
        r#"tell application "Mail"
    activate
    delay 0.5
    set msg to make new outgoing message with properties {{subject:"{}", content:"{}", sender:"{}", visible:true}}
    tell msg
        make new to recipient with properties {{address:"{}"}}
    end tell
end tell"#,
        escaped_subject, escaped_body, escaped_from, escaped_to
    );

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
                Grant access in: System Settings > Privacy & Security > Automation\n\
                Enable access for your terminal to control Mail."
            );
        }

        if stderr_lower.contains("can't get account") || stderr_lower.contains("no account") {
            anyhow::bail!(
                "Mail.app account not found.\n\n\
                Open Mail.app and verify the account is configured correctly."
            );
        }

        anyhow::bail!("Send failed: {}", stderr.trim());
    }

    Ok(())
}

/// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
fn send_email_via_mail_app(
    _from_addr: &str,
    _to_addr: &str,
    _subject: &str,
    _body: &str,
) -> Result<()> {
    anyhow::bail!("Email sending is only available on macOS.")
}

/// Escape a string for use in AppleScript
/// Handles special characters that could break the script
fn escape_applescript(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            // Filter out null bytes and other control chars that could cause issues
            '\0'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f' => {}
            _ => result.push(c),
        }
    }
    result
}

/// Show an error message and wait for keypress
pub fn show_email_error(message: &str) -> Result<()> {
    clear_screen()?;
    println!("Error: {}\n", message);
    print!("[q]uit: ");
    io::stdout().flush()?;
    wait_for_key()?;
    Ok(())
}
