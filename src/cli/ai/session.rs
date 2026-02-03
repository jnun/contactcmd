//! AI Chat Session management
//!
//! # SECURITY: DATA FIREWALL
//!
//! **THE AI HAS NO ACCESS TO USER DATA.**
//!
//! This session manager enforces the data firewall:
//!
//! ## Input to AI:
//! - User's natural language message only (the `&str` they typed)
//! - Conversation history (prior messages in this session)
//! - Tool definitions (command schemas, not data)
//!
//! ## Output from AI:
//! - Command suggestions (e.g., "/search john in alabama")
//! - Natural language responses
//!
//! ## What is NEVER sent to AI:
//! - Contact data (Person, Email, Phone, Address)
//! - Message history (iMessage/SMS content)
//! - Search results
//! - Any data from the database
//!
//! The `Database` import here is ONLY for loading AI configuration
//! (API keys, model settings). It is never used to query user data.

use super::{
    executor::ToolExecutor,
    provider::AiProvider,
    tools::get_all_tools,
    AiConfig, AiProviderType, ChatMessage, RemoteProvider,
};
use crate::db::Database;
use anyhow::{anyhow, Result};

/// System prompt loaded from markdown file at compile time
/// This allows easy editing without recompiling for content changes during development,
/// while still being compiled into the binary for production.
const SYSTEM_PROMPT: &str = include_str!("instructions.md");

/// Maximum number of tool call iterations
const MAX_TOOL_ITERATIONS: usize = 5;

/// Result from AI chat - contains both display text and extracted command
pub struct AiChatResult {
    /// Text to display to user (AI's explanation)
    pub display_text: String,
    /// Command to execute (extracted directly from tool calls, not parsed from text)
    pub command: Option<String>,
}

/// Manages a conversation session with the AI
///
/// IMPORTANT: This session has NO access to user data.
/// The AI only suggests commands; it cannot see contacts or messages.
pub struct AiChatSession {
    messages: Vec<ChatMessage>,
    config: AiConfig,
}

impl AiChatSession {
    /// Create a new AI chat session
    pub fn new(config: AiConfig) -> Self {
        let messages = vec![ChatMessage::system(SYSTEM_PROMPT)];
        Self { messages, config }
    }

    /// Load configuration and create a session, returning None if not configured
    pub fn from_database(db: &Database) -> Result<Option<Self>> {
        let config = AiConfig::load(db)?;
        if !config.is_configured() {
            return Ok(None);
        }
        Ok(Some(Self::new(config)))
    }

    /// Check if the session is ready to use
    pub fn is_ready(&self) -> bool {
        self.config.is_configured()
    }

    /// Get provider type for display
    pub fn provider_type(&self) -> &AiProviderType {
        &self.config.provider_type
    }

    /// Process a user message and return structured result
    ///
    /// IMPORTANT: The AI has NO access to user data. Tool calls only
    /// generate command suggestions, not actual data queries.
    ///
    /// Returns `AiChatResult` with:
    /// - `command`: Captured directly from tool execution (reliable)
    /// - `display_text`: AI's explanation (for display only, not control flow)
    pub fn chat(&mut self, user_message: &str) -> Result<AiChatResult> {
        // Add user message to history
        self.messages.push(ChatMessage::user(user_message));

        // Create provider based on config
        let provider = self.create_provider()?;
        let tools = get_all_tools();
        let executor = ToolExecutor::new();

        let mut iterations = 0;
        // Capture command directly from tool execution - don't rely on AI echoing it
        let mut captured_command: Option<String> = None;

        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                return Err(anyhow!("Too many iterations"));
            }

            // Get completion from AI
            let response = provider.complete(&self.messages, &tools)?;

            // Handle tool calls (these only generate command suggestions)
            if !response.tool_calls.is_empty() {
                self.messages
                    .push(ChatMessage::assistant_with_tool_calls(response.tool_calls.clone()));

                for tool_call in &response.tool_calls {
                    // Execute returns a command suggestion, NOT data
                    let result = executor.execute(tool_call)?;

                    // CAPTURE the command directly - this is the reliable path
                    if !result.command.is_empty() {
                        captured_command = Some(result.command.clone());
                    }

                    // Tell AI the command was noted and to stop calling tools
                    let tool_response = format!(
                        "Command queued: {}. STOP - do not call more tools. Respond to user now with a brief explanation.",
                        result.command
                    );
                    self.messages
                        .push(ChatMessage::tool_result(&tool_call.id, &tool_response));
                }

                continue;
            }

            // Get AI's display text (explanation only - not used for control flow)
            let display_text = response.content.unwrap_or_default();

            if !display_text.is_empty() {
                self.messages.push(ChatMessage::assistant(&display_text));
            }

            return Ok(AiChatResult {
                display_text,
                command: captured_command,
            });
        }
    }

    /// Create the appropriate provider based on configuration
    fn create_provider(&self) -> Result<Box<dyn AiProvider>> {
        match self.config.provider_type {
            AiProviderType::None => Err(anyhow!("AI not configured")),
            AiProviderType::Remote => {
                let provider = RemoteProvider::new(&self.config)?;
                Ok(Box::new(provider))
            }
            #[cfg(feature = "local-ai")]
            AiProviderType::Local => {
                use super::LocalProvider;
                let provider = LocalProvider::new(&self.config)?;
                Ok(Box::new(provider))
            }
        }
    }

    /// Clear conversation history (keep system prompt)
    pub fn clear_history(&mut self) {
        self.messages.truncate(1);
    }

    /// Get current message count (excluding system prompt)
    pub fn message_count(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }

    /// Provide feedback to the AI about command results
    ///
    /// Call this after executing a command to let the AI know what happened.
    /// If results were poor, the AI can suggest alternatives.
    pub fn provide_feedback(&mut self, feedback: CommandFeedback) -> Result<Option<AiChatResult>> {
        let feedback_msg = feedback.to_message();

        // Add feedback as a system-style user message
        self.messages.push(ChatMessage::user(format!("[System feedback]: {}", feedback_msg)));

        // If no results and user might want alternatives, ask AI to suggest
        if feedback.should_suggest_alternative() {
            let provider = self.create_provider()?;
            let tools = get_all_tools();
            let executor = ToolExecutor::new();

            // Get AI's alternative suggestion
            let response = provider.complete(&self.messages, &tools)?;

            // Handle tool calls (AI might use suggest_search for alternative)
            if !response.tool_calls.is_empty() {
                self.messages
                    .push(ChatMessage::assistant_with_tool_calls(response.tool_calls.clone()));

                for tool_call in &response.tool_calls {
                    let result = executor.execute(tool_call)?;

                    if !result.command.is_empty() {
                        // Found alternative command via tool
                        self.messages.push(ChatMessage::tool_result(
                            &tool_call.id,
                            format!("Alternative command: {}", result.command),
                        ));

                        return Ok(Some(AiChatResult {
                            display_text: format!("Trying simpler search..."),
                            command: Some(result.command),
                        }));
                    }
                }
            }

            // Handle text response (AI might suggest command in text)
            if let Some(content) = response.content {
                self.messages.push(ChatMessage::assistant(&content));
                return Ok(Some(AiChatResult {
                    display_text: content,
                    command: None, // Command will be extracted from text by caller
                }));
            }
        }

        Ok(None)
    }
}

/// Feedback about command execution results
#[derive(Debug, Clone)]
pub struct CommandFeedback {
    /// The command that was executed
    pub command: String,
    /// Number of results found (for search commands)
    pub result_count: Option<usize>,
    /// Action taken (e.g., "opened", "listed", "browsing")
    pub action: FeedbackAction,
    /// Original user query (for context)
    pub original_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackAction {
    /// Search found results
    Found,
    /// Search found nothing
    NoResults,
    /// Opened a single contact
    OpenedContact,
    /// Browsing multiple contacts
    Browsing,
    /// Listed all contacts
    Listed,
    /// Showed messages
    ShowedMessages,
    /// Error occurred
    Error(String),
}

impl CommandFeedback {
    pub fn search_results(command: String, count: usize, original_query: Option<String>) -> Self {
        Self {
            command,
            result_count: Some(count),
            action: if count == 0 { FeedbackAction::NoResults } else { FeedbackAction::Found },
            original_query,
        }
    }

    pub fn no_results(command: String, original_query: Option<String>) -> Self {
        Self {
            command,
            result_count: Some(0),
            action: FeedbackAction::NoResults,
            original_query,
        }
    }

    pub fn opened_contact(command: String) -> Self {
        Self {
            command,
            result_count: Some(1),
            action: FeedbackAction::OpenedContact,
            original_query: None,
        }
    }

    fn to_message(&self) -> String {
        match &self.action {
            FeedbackAction::Found => {
                format!("Found {} contacts for '{}'",
                    self.result_count.unwrap_or(0),
                    self.command)
            }
            FeedbackAction::NoResults => {
                let mut msg = format!("No matches for '{}'. ", self.command);
                if let Some(ref q) = self.original_query {
                    msg.push_str(&format!(
                        "The user originally asked: \"{}\". Suggest a simpler search.", q
                    ));
                } else {
                    msg.push_str("Suggest a simpler or broader search.");
                }
                msg
            }
            FeedbackAction::OpenedContact => {
                format!("Opened contact from '{}'", self.command)
            }
            FeedbackAction::Browsing => {
                format!("User is browsing {} contacts", self.result_count.unwrap_or(0))
            }
            FeedbackAction::Listed => {
                format!("Listed {} contacts", self.result_count.unwrap_or(0))
            }
            FeedbackAction::ShowedMessages => {
                format!("Showed messages for '{}'", self.command)
            }
            FeedbackAction::Error(e) => {
                format!("Error executing '{}': {}", self.command, e)
            }
        }
    }

    fn should_suggest_alternative(&self) -> bool {
        matches!(self.action, FeedbackAction::NoResults)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let config = AiConfig::default();
        let session = AiChatSession::new(config);

        // Should have system prompt
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_clear_history() {
        let config = AiConfig::default();
        let mut session = AiChatSession::new(config);

        // Manually add some messages
        session.messages.push(ChatMessage::user("test"));
        session.messages.push(ChatMessage::assistant("response"));

        assert_eq!(session.message_count(), 2);

        session.clear_history();

        // Should only have system prompt left
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.message_count(), 0);
    }
}
