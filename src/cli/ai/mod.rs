//! Conversational AI module for ContactCMD
//!
//! # SECURITY ARCHITECTURE - DATA FIREWALL
//!
//! **THE AI HAS NO ACCESS TO USER DATA.**
//!
//! This module is intentionally designed with a strict firewall:
//!
//! ## What the AI CAN do:
//! - Receive natural language input from the user
//! - Suggest CLI commands (e.g., "/search john in alabama")
//! - Access its own configuration (API keys, model settings)
//!
//! ## What the AI CANNOT do:
//! - Query the database for contacts, messages, or any user data
//! - See search results or contact details
//! - Access any Person, Contact, Message, Email, Phone, or Address data
//! - Execute commands (only suggest them)
//!
//! ## Enforcement:
//! - `ToolExecutor` has NO database parameter - it cannot query data
//! - `AiChatSession::chat()` takes only the user's text message
//! - Tool results are command strings, not data
//! - The CLI executes commands AFTER AI is done, results never sent back
//!
//! ## Audit checklist:
//! - [ ] No `Database` parameter in `ToolExecutor::new()` or `execute()`
//! - [ ] No `Person`, `Contact`, `Message` imports in executor.rs
//! - [ ] `AiChatSession::chat()` takes only `&str` message
//! - [ ] Tool results contain only `command` and `explanation` strings
//!
//! Provides a natural language interface to CLI commands through either:
//! - Remote API providers (OpenAI-compatible endpoints)
//! - Local models via llama-cpp-2 (optional, requires `local-ai` feature)

mod config;
mod executor;
#[cfg(feature = "local-ai")]
mod local;
#[cfg(feature = "local-ai")]
mod models;
mod provider;
mod remote;
mod session;
mod tools;

pub use config::{check_ram_for_model, AiConfig, AiProviderType, LocalModelId, SETTING_AI_API_ENDPOINT, SETTING_AI_API_KEY, SETTING_AI_API_URL, SETTING_AI_MODEL, SETTING_AI_PROVIDER};
pub use executor::{ToolExecutor, ToolResult};
#[cfg(feature = "local-ai")]
pub use local::LocalProvider;
#[cfg(feature = "local-ai")]
pub use models::LocalModel;
pub use provider::AiProvider;
pub use remote::RemoteProvider;
pub use session::{AiChatResult, AiChatSession, CommandFeedback, FeedbackAction};
pub use tools::{get_all_tools, AiTool, ToolParameter};

use serde::{Deserialize, Serialize};

/// Role of a message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// A tool call requested by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Response from an AI provider
#[derive(Debug, Clone)]
pub struct AiResponse {
    /// Text content from the assistant
    pub content: Option<String>,
    /// Tool calls requested by the assistant
    pub tool_calls: Vec<ToolCall>,
    /// Whether the response is complete (no more tool calls needed)
    pub is_complete: bool,
    /// Finish reason from the API
    pub finish_reason: Option<String>,
}

impl AiResponse {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: Vec::new(),
            is_complete: true,
            finish_reason: Some("stop".to_string()),
        }
    }

    pub fn with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: None,
            tool_calls,
            is_complete: false,
            finish_reason: Some("tool_calls".to_string()),
        }
    }
}
