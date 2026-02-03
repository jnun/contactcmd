//! Message execution for the gateway.
//!
//! Sends approved messages via email, SMS, or iMessage.

use anyhow::Result;

use crate::db::gateway::QueueEntry;
use crate::db::Database;

/// Execute sending a queued message.
pub fn execute_send(db: &Database, entry: &QueueEntry) -> Result<()> {
    match entry.channel.as_str() {
        "email" => send_email(db, entry),
        "sms" | "imessage" => send_message(entry),
        other => anyhow::bail!("Unknown channel: {}", other),
    }
}

/// Send email via Gmail API.
fn send_email(db: &Database, entry: &QueueEntry) -> Result<()> {
    use base64::Engine;

    use crate::cli::google_auth::{get_google_email, is_google_auth_configured, refresh_access_token_if_needed};

    if !is_google_auth_configured(db) {
        anyhow::bail!("Gmail not configured. Run setup first.");
    }

    let from_addr = get_google_email(db).unwrap_or_default();
    let to_addr = &entry.recipient_address;
    let subject = entry.subject.as_deref().unwrap_or("(no subject)");
    let body = &entry.body;

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

/// Send SMS or iMessage.
#[cfg(target_os = "macos")]
fn send_message(entry: &QueueEntry) -> Result<()> {
    use std::process::Command;

    let recipient = &entry.recipient_address;
    let message = &entry.body;

    // Escape for AppleScript
    let escaped_message = message.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_recipient = recipient.replace('\\', "\\\\").replace('"', "\\\"");

    // Determine if this looks like a phone number
    let is_phone = recipient.chars().any(|c| c.is_ascii_digit()) && !recipient.contains('@');

    // Build AppleScript based on channel preference
    let script = if entry.channel == "sms" {
        // Force SMS
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
    } else if entry.channel == "imessage" || !is_phone {
        // Force iMessage (or default for email addresses)
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
    } else {
        // Auto-detect: try iMessage first, fall back to SMS
        format!(
            r#"
            tell application "Messages"
                try
                    set imsgService to 1st account whose service type = iMessage
                    set targetBuddy to participant "{0}" of imsgService
                    send "{1}" to targetBuddy
                    return "sent"
                end try

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
                "Permission required. Grant access in: System Settings > Privacy & Security > Accessibility"
            );
        }
        if stderr_lower.contains("can't get account") || stderr_lower.contains("no account") {
            anyhow::bail!("Messages.app is not set up. Open Messages.app and sign in first.");
        }
        anyhow::bail!("Send failed: {}", stderr.trim());
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_message(_entry: &QueueEntry) -> Result<()> {
    anyhow::bail!("SMS/iMessage sending is only available on macOS")
}
