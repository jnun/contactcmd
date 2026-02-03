//! AI Provider trait definition
//!
//! Defines the interface that all AI providers must implement.

use super::{AiResponse, AiTool, ChatMessage};
use anyhow::Result;

/// Trait for AI providers that can generate completions
pub trait AiProvider: Send + Sync {
    /// Generate a completion for the given conversation
    ///
    /// # Arguments
    /// * `messages` - The conversation history
    /// * `tools` - Available tools the AI can call
    ///
    /// # Returns
    /// An `AiResponse` containing either text content or tool calls
    fn complete(&self, messages: &[ChatMessage], tools: &[AiTool]) -> Result<AiResponse>;

    /// Get the name of this provider for display purposes
    fn name(&self) -> &str;

    /// Check if this provider is ready to use
    fn is_ready(&self) -> bool;
}
