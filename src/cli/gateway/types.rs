//! Types for the communication gateway.

use serde::{Deserialize, Serialize};

/// Communication channel for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GatewayChannel {
    Sms,
    IMessage,
    Email,
}

impl std::fmt::Display for GatewayChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayChannel::Sms => write!(f, "sms"),
            GatewayChannel::IMessage => write!(f, "imessage"),
            GatewayChannel::Email => write!(f, "email"),
        }
    }
}

impl std::str::FromStr for GatewayChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sms" => Ok(GatewayChannel::Sms),
            "imessage" => Ok(GatewayChannel::IMessage),
            "email" => Ok(GatewayChannel::Email),
            _ => Err(format!("unknown channel: {}", s)),
        }
    }
}

/// Message priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Urgent,
    High,
    #[default]
    Normal,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Urgent => write!(f, "urgent"),
            Priority::High => write!(f, "high"),
            Priority::Normal => write!(f, "normal"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "urgent" => Ok(Priority::Urgent),
            "high" => Ok(Priority::High),
            "normal" => Ok(Priority::Normal),
            "low" => Ok(Priority::Low),
            _ => Err(format!("unknown priority: {}", s)),
        }
    }
}

/// Queue entry status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueueStatus {
    Pending,
    Flagged,
    Approved,
    Denied,
    Sent,
    Failed,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStatus::Pending => write!(f, "pending"),
            QueueStatus::Flagged => write!(f, "flagged"),
            QueueStatus::Approved => write!(f, "approved"),
            QueueStatus::Denied => write!(f, "denied"),
            QueueStatus::Sent => write!(f, "sent"),
            QueueStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for QueueStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(QueueStatus::Pending),
            "flagged" => Ok(QueueStatus::Flagged),
            "approved" => Ok(QueueStatus::Approved),
            "denied" => Ok(QueueStatus::Denied),
            "sent" => Ok(QueueStatus::Sent),
            "failed" => Ok(QueueStatus::Failed),
            _ => Err(format!("unknown status: {}", s)),
        }
    }
}

// ========== API Request/Response Types ==========

/// Request to queue a message for sending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequest {
    pub channel: GatewayChannel,
    pub recipient_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
    #[serde(default)]
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Response after queueing a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResponse {
    pub action_id: String,
    pub status: QueueStatus,
}

/// Response for action status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStatusResponse {
    pub action_id: String,
    pub status: QueueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
}

/// Queue entry for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryResponse {
    pub id: String,
    pub channel: String,
    pub recipient_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
    pub priority: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_context: Option<serde_json::Value>,
    pub created_at: String,
    pub agent_name: String,
}

/// List of pending queue entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueListResponse {
    pub entries: Vec<QueueEntryResponse>,
    pub total: usize,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> GatewayApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

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
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub pending_count: i64,
    pub version: String,
}

/// Rate limit exceeded error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitErrorResponse {
    pub error: String,
    pub retry_after_seconds: i64,
    pub limit_type: String,
    pub current_count: i64,
    pub limit: i32,
}

/// Recipient not allowed error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistErrorResponse {
    pub error: String,
    pub allowed_patterns: Vec<String>,
}

/// Content blocked by filter error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockedErrorResponse {
    pub error: String,
    pub filter: String,
    pub description: String,
}

/// Contact consent denied error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentDeniedErrorResponse {
    pub error: String,
    pub recipient: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_serialization() {
        let channel = GatewayChannel::Email;
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, r#""email""#);

        let channel: GatewayChannel = serde_json::from_str(r#""sms""#).unwrap();
        assert_eq!(channel, GatewayChannel::Sms);
    }

    #[test]
    fn test_send_request() {
        let req = SendRequest {
            channel: GatewayChannel::Email,
            recipient_address: "test@example.com".to_string(),
            recipient_name: Some("Test User".to_string()),
            subject: Some("Hello".to_string()),
            body: "This is a test".to_string(),
            priority: Priority::Normal,
            context: Some(serde_json::json!({"reason": "calendar followup"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""channel":"email""#));
        assert!(json.contains(r#""priority":"normal""#));
    }

    #[test]
    fn test_api_response() {
        let resp: GatewayApiResponse<String> = GatewayApiResponse::ok("success".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("success".to_string()));

        let resp: GatewayApiResponse<String> = GatewayApiResponse::err("failed");
        assert!(!resp.success);
        assert_eq!(resp.error, Some("failed".to_string()));
    }
}
