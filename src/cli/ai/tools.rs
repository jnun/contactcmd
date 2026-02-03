//! AI tool definitions
//!
//! IMPORTANT: The AI has NO access to user data. These tools only generate
//! command suggestions that the user can then execute.

use serde::{Deserialize, Serialize};

/// Definition of a tool the AI can call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl ToolParameter {
    pub fn required(name: &str, description: &str, param_type: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            param_type: param_type.to_string(),
            required: true,
            enum_values: None,
        }
    }

    pub fn optional(name: &str, description: &str, param_type: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            param_type: param_type.to_string(),
            required: false,
            enum_values: None,
        }
    }
}

/// Get all available tools for the AI
///
/// IMPORTANT: These tools generate command suggestions only.
/// The AI has NO access to user data - it cannot see contacts, messages, etc.
pub fn get_all_tools() -> Vec<AiTool> {
    vec![
        suggest_search_tool(),
        suggest_list_tool(),
        suggest_show_tool(),
        suggest_messages_tool(),
        suggest_recent_tool(),
        suggest_browse_tool(),
    ]
}

fn suggest_search_tool() -> AiTool {
    AiTool {
        name: "suggest_search".to_string(),
        description: "Search contacts. Use location for cities/states, organization for companies, name for people, query for general terms.".to_string(),
        parameters: vec![
            ToolParameter::optional("query", "General search terms (searches all fields)", "string"),
            ToolParameter::optional("name", "Person's name", "string"),
            ToolParameter::optional("location", "City or state (e.g., 'miami', 'texas')", "string"),
            ToolParameter::optional("organization", "Company name (e.g., 'google', 'att')", "string"),
        ],
    }
}

fn suggest_list_tool() -> AiTool {
    AiTool {
        name: "suggest_list".to_string(),
        description: "Suggest the list command to show all contacts.".to_string(),
        parameters: vec![],
    }
}

fn suggest_show_tool() -> AiTool {
    AiTool {
        name: "suggest_show".to_string(),
        description: "Suggest showing a specific contact's details.".to_string(),
        parameters: vec![
            ToolParameter::required("name", "Contact name to show", "string"),
        ],
    }
}

fn suggest_messages_tool() -> AiTool {
    AiTool {
        name: "suggest_messages".to_string(),
        description: "Suggest viewing messages with a contact.".to_string(),
        parameters: vec![
            ToolParameter::required("contact", "Contact name to view messages with", "string"),
        ],
    }
}

fn suggest_recent_tool() -> AiTool {
    AiTool {
        name: "suggest_recent".to_string(),
        description: "Suggest viewing recently messaged contacts (iMessage/SMS).".to_string(),
        parameters: vec![
            ToolParameter::optional("days", "Number of days to look back (default: 7)", "integer"),
        ],
    }
}

fn suggest_browse_tool() -> AiTool {
    AiTool {
        name: "suggest_browse".to_string(),
        description: "Suggest browsing previous search results in TUI.".to_string(),
        parameters: vec![],
    }
}
