//! Tool executor for AI tool calls
//!
//! # SECURITY: DATA FIREWALL
//!
//! **THIS MODULE HAS NO ACCESS TO USER DATA.**
//!
//! This executor is intentionally designed WITHOUT any database access:
//! - No `Database` parameter in `new()` or any method
//! - No imports of `Person`, `Contact`, `Message`, or any data models
//! - Returns only command suggestion strings, never actual data
//!
//! The firewall is enforced by design: there is simply no way to pass
//! user data into this module. Any attempt to add database access
//! should be rejected in code review.
//!
//! ## Allowed:
//! - Parsing tool call arguments (query terms, contact names as search input)
//! - Generating command strings like "/search john in alabama"
//!
//! ## Forbidden:
//! - Any database queries
//! - Any access to contact/message data
//! - Returning actual user data in ToolResult

use super::ToolCall;
use anyhow::{anyhow, Result};

// SECURITY: Compile-time documentation that this module has no database access.
// If you're reading this and considering adding Database here, DON'T.
// The AI must never see user data. Commands are executed by the CLI after
// AI processing is complete, and results are shown directly to the user.

/// Result of executing a tool - only contains command suggestions
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The suggested command for the user to run
    pub command: String,
    /// Human-readable explanation
    pub explanation: String,
}

impl ToolResult {
    pub fn new(command: impl Into<String>, explanation: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            explanation: explanation.into(),
        }
    }
}

/// Executor that generates command suggestions (NO data access)
pub struct ToolExecutor;

impl ToolExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a tool call - returns a command suggestion only
    ///
    /// IMPORTANT: This does NOT access any user data. It only translates
    /// the AI's tool call into a CLI command suggestion.
    pub fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        let tool_name = &tool_call.function.name;
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .unwrap_or(serde_json::json!({}));

        match tool_name.as_str() {
            "suggest_search" => self.suggest_search(&args),
            "suggest_list" => Ok(ToolResult::new("/list", "List all contacts")),
            "suggest_show" => self.suggest_show(&args),
            "suggest_messages" => self.suggest_messages(&args),
            "suggest_recent" => self.suggest_recent(&args),
            "suggest_browse" => Ok(ToolResult::new("/browse", "Browse previous results in TUI")),
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    fn suggest_search(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let query = args.get("query").and_then(|v| v.as_str());
        let name = args.get("name").and_then(|v| v.as_str());
        let location = args.get("location").and_then(|v| v.as_str());
        let organization = args.get("organization").and_then(|v| v.as_str());

        // Build the search command
        // Note: "in <loc>" and "at <org>" syntax requires something BEFORE them
        // e.g., "/search john in miami" is valid, "/search in miami" is NOT

        let mut base_terms = Vec::new();
        let mut location_term: Option<String> = None;
        let mut org_term: Option<String> = None;

        if let Some(n) = name {
            let n = n.trim();
            if !n.is_empty() {
                base_terms.push(n.to_string());
            }
        }
        if let Some(q) = query {
            let q = q.trim();
            if !q.is_empty() {
                base_terms.push(q.to_string());
            }
        }
        if let Some(loc) = location {
            // Strip any accidental "in " prefix the AI might have added
            let loc = loc.trim();
            let loc = loc.strip_prefix("in ").unwrap_or(loc).trim();
            if !loc.is_empty() {
                location_term = Some(loc.to_string());
            }
        }
        if let Some(org) = organization {
            // Strip any accidental "at " prefix the AI might have added
            let org = org.trim();
            let org = org.strip_prefix("at ").unwrap_or(org).trim();
            if !org.is_empty() {
                org_term = Some(org.to_string());
            }
        }

        // Build the final query
        // If we have base terms, we can use "in <loc>" and "at <org>" syntax
        // If we ONLY have location/org, just search for them directly (no "in"/"at" prefix)
        let mut parts = Vec::new();

        if !base_terms.is_empty() {
            parts.push(base_terms.join(" "));
            // Can use "in" and "at" syntax since we have base terms
            if let Some(loc) = location_term {
                parts.push(format!("in {}", loc));
            }
            if let Some(org) = org_term {
                parts.push(format!("at {}", org));
            }
        } else {
            // No base terms - just search for location/org directly
            if let Some(loc) = location_term {
                parts.push(loc);
            }
            if let Some(org) = org_term {
                parts.push(org);
            }
        }

        if parts.is_empty() {
            return Ok(ToolResult::new("/search", "Search for contacts"));
        }

        let search_query = parts.join(" ");
        Ok(ToolResult::new(
            format!("/search {}", search_query),
            format!("Search for: {}", search_query),
        ))
    }

    fn suggest_show(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        Ok(ToolResult::new(
            format!("/search {}", name),
            format!("Search for {} then use /browse to view details", name),
        ))
    }

    fn suggest_messages(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let contact = args.get("contact").and_then(|v| v.as_str()).unwrap_or("");
        Ok(ToolResult::new(
            format!("/messages {}", contact),
            format!("View messages with {}", contact),
        ))
    }

    fn suggest_recent(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let days = args.get("days")
            .and_then(|v| v.as_i64())
            .map(|d| d as u32)
            .unwrap_or(7);

        if days == 7 {
            Ok(ToolResult::new("/recent", "View recently messaged contacts"))
        } else {
            Ok(ToolResult::new(
                format!("/recent {}", days),
                format!("View contacts messaged in last {} days", days),
            ))
        }
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}
