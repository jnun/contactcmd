//! Message types for the Moltbot bridge protocol.

use serde::{Deserialize, Serialize};

/// Communication channel for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeChannel {
    /// iMessage
    IMessage,
    /// SMS
    Sms,
}

impl std::fmt::Display for BridgeChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeChannel::IMessage => write!(f, "imessage"),
            BridgeChannel::Sms => write!(f, "sms"),
        }
    }
}

/// Message received on Mac, forwarded to Moltbot.
/// contactcmd → Moltbot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Unique message ID
    pub id: String,
    /// Sender phone number or identifier
    pub sender: String,
    /// Message content
    pub content: String,
    /// Channel the message arrived on
    pub channel: BridgeChannel,
    /// Unix timestamp when received
    pub timestamp: i64,
    /// Optional sender display name if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
}

/// Message from Moltbot to send via Mac.
/// Moltbot → contactcmd
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Unique message ID
    pub id: String,
    /// Recipient phone number or identifier
    pub recipient: String,
    /// Message content to send
    pub content: String,
    /// Channel to send on
    pub channel: BridgeChannel,
    /// Optional reference to message being replied to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

/// Initial handshake from contactcmd to Moltbot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// Protocol version
    pub version: String,
    /// Hostname of the Mac
    pub hostname: String,
    /// Capabilities supported by this bridge
    pub capabilities: Vec<String>,
}

impl Default for HandshakeRequest {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            capabilities: vec!["imessage".to_string(), "sms".to_string()],
        }
    }
}

/// Handshake response from Moltbot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Whether handshake was accepted
    pub accepted: bool,
    /// Session token for subsequent requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    /// Error message if not accepted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeApiResponse<T> {
    /// Whether the request succeeded
    pub success: bool,
    /// Response data if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> BridgeApiResponse<T> {
    /// Create a success response with data.
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service status
    pub status: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Protocol version
    pub version: String,
}

/// Kill request to stop the bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillRequest {
    /// Reason for stopping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_serialization() {
        let channel = BridgeChannel::IMessage;
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, r#""imessage""#);

        let channel = BridgeChannel::Sms;
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, r#""sms""#);
    }

    #[test]
    fn test_inbound_message() {
        let msg = InboundMessage {
            id: "msg-123".to_string(),
            sender: "+1234567890".to_string(),
            content: "Hello, world!".to_string(),
            channel: BridgeChannel::IMessage,
            timestamp: 1700000000,
            sender_name: Some("John Doe".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""sender":"+1234567890""#));
        assert!(json.contains(r#""channel":"imessage""#));
    }

    #[test]
    fn test_api_response() {
        let resp: BridgeApiResponse<String> = BridgeApiResponse::ok("success".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("success".to_string()));
        assert!(resp.error.is_none());

        let resp: BridgeApiResponse<String> = BridgeApiResponse::err("failed");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("failed".to_string()));
    }
}
