//! Webhook notifications for gateway status changes.
//!
//! Sends HTTP POST requests to configured webhook URLs when message status changes.

use serde::Serialize;
use std::time::Duration;

use crate::db::Database;

/// Webhook payload sent when message status changes.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub action_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub recipient: String,
    pub channel: String,
}

/// Result of a webhook delivery attempt.
#[derive(Debug)]
pub enum WebhookResult {
    /// Successfully delivered (HTTP 2xx response).
    Delivered,
    /// No webhook configured for this API key.
    NoWebhook,
    /// Delivery failed (logged but not blocking).
    Failed(String),
}

/// Send webhook notification for a status change.
///
/// This function is non-blocking for errors - webhook failures are logged
/// but don't prevent the status change from completing.
pub fn notify_status_change(
    db: &Database,
    api_key_id: &str,
    action_id: &str,
    status: &str,
    recipient: &str,
    channel: &str,
    sent_at: Option<&str>,
    error_message: Option<&str>,
) -> WebhookResult {
    // Get webhook URL for this API key
    let webhook_url = match db.get_api_key_webhook(api_key_id) {
        Ok(Some(url)) => url,
        Ok(None) => return WebhookResult::NoWebhook,
        Err(e) => {
            eprintln!("Warning: Failed to get webhook URL: {}", e);
            return WebhookResult::Failed(e.to_string());
        }
    };

    // Build payload
    let payload = WebhookPayload {
        action_id: action_id.to_string(),
        status: status.to_string(),
        sent_at: sent_at.map(String::from),
        error_message: error_message.map(String::from),
        recipient: recipient.to_string(),
        channel: channel.to_string(),
    };

    // Send webhook (blocking HTTP call with timeout)
    match send_webhook(&webhook_url, &payload) {
        Ok(()) => WebhookResult::Delivered,
        Err(e) => {
            eprintln!(
                "Warning: Webhook delivery failed for action {}: {}",
                action_id, e
            );
            WebhookResult::Failed(e)
        }
    }
}

/// Send HTTP POST to webhook URL.
fn send_webhook(url: &str, payload: &WebhookPayload) -> Result<(), String> {
    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid webhook URL: must start with http:// or https://".to_string());
    }

    let json_body = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    // Use blocking reqwest client with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "contactcmd-gateway/1.0")
        .body(json_body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("Webhook returned HTTP {}", status.as_u16()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            action_id: "abc123".to_string(),
            status: "sent".to_string(),
            sent_at: Some("2026-02-03T12:00:00Z".to_string()),
            error_message: None,
            recipient: "test@example.com".to_string(),
            channel: "email".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains(r#""action_id":"abc123""#));
        assert!(json.contains(r#""status":"sent""#));
        assert!(json.contains(r#""sent_at":"2026-02-03T12:00:00Z""#));
        assert!(!json.contains("error_message")); // Should be skipped when None
        assert!(json.contains(r#""recipient":"test@example.com""#));
        assert!(json.contains(r#""channel":"email""#));
    }

    #[test]
    fn test_webhook_payload_with_error() {
        let payload = WebhookPayload {
            action_id: "def456".to_string(),
            status: "failed".to_string(),
            sent_at: None,
            error_message: Some("Connection refused".to_string()),
            recipient: "+15551234567".to_string(),
            channel: "sms".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains(r#""status":"failed""#));
        assert!(json.contains(r#""error_message":"Connection refused""#));
        assert!(!json.contains("sent_at")); // Should be skipped when None
    }

    #[test]
    fn test_no_webhook_configured() {
        let db = Database::open_memory().unwrap();

        // Create an API key without webhook
        db.insert_api_key("key-1", "Test Agent", "hash123", "gw_abc")
            .unwrap();

        let result = notify_status_change(
            &db,
            "key-1",
            "action-1",
            "sent",
            "test@example.com",
            "email",
            Some("2026-02-03T12:00:00Z"),
            None,
        );

        assert!(matches!(result, WebhookResult::NoWebhook));
    }

    #[test]
    fn test_invalid_webhook_url() {
        let payload = WebhookPayload {
            action_id: "test".to_string(),
            status: "sent".to_string(),
            sent_at: None,
            error_message: None,
            recipient: "test@example.com".to_string(),
            channel: "email".to_string(),
        };

        // Test invalid URL
        let result = send_webhook("not-a-valid-url", &payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with http"));
    }
}
