//! Shared UI primitives for contactcmd
//!
//! Design principles:
//! - Minimal: Show only what's needed
//! - Clean: No decorative borders or lines
//! - Consistent: Same patterns everywhere
//!
//! Conventions:
//! - Prompts: lowercase with colon and space: `search: `
//! - Navigation hints: arrows in brackets: `[↑/↓]` vertical, `[←/→]` horizontal
//! - Feedback: single word when possible: `Saved.`

use anyhow::Result;
use crossterm::{
    cursor,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use inquire::{ui::RenderConfig, Confirm, InquireError, Select, Text};
use std::io::{self, Write};

/// Clear the terminal screen and move cursor to top-left
pub fn clear_screen() -> Result<()> {
    let mut stdout = io::stdout();
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.flush()?;
    Ok(())
}

/// Get terminal dimensions, defaulting to 80x24 if unavailable
/// Works across: macOS Terminal/iTerm2, Linux terminals, Windows Terminal,
/// tmux, screen, SSH sessions. Falls back safely for pipes/non-TTY.
pub fn term_size() -> (usize, usize) {
    crossterm::terminal::size()
        .map(|(w, h)| (w as usize, h as usize))
        .unwrap_or((80, 24))
}

/// Get number of visible content lines for scrollable lists.
/// Call this inside your display loop to handle terminal resize.
/// Accounts for header (2 lines) and status bar (2 lines).
pub fn visible_lines() -> usize {
    let (_, height) = term_size();
    height.saturating_sub(4).max(5) // At least 5 lines of content
}

/// Get a minimal render config for inquire prompts
pub fn minimal_render_config() -> RenderConfig<'static> {
    RenderConfig::default_colored()
        .with_prompt_prefix(inquire::ui::Styled::new(""))
        .with_answered_prompt_prefix(inquire::ui::Styled::new(""))
}

/// Display a selection menu and return the chosen index
pub fn select<T: ToString + Clone>(prompt: &str, options: &[T]) -> Result<Option<usize>> {
    if options.is_empty() {
        return Ok(None);
    }

    let items: Vec<String> = options.iter().map(|o| o.to_string()).collect();

    let result = Select::new(prompt, items)
        .with_render_config(minimal_render_config())
        .with_page_size(visible_lines())
        .with_vim_mode(true)
        .prompt_skippable()?;

    match result {
        Some(selected) => {
            // Find the index of the selected item
            let idx = options
                .iter()
                .position(|o| o.to_string() == selected)
                .unwrap_or(0);
            Ok(Some(idx))
        }
        None => Ok(None),
    }
}

/// Prompt for text input with optional default value
pub fn text_input(prompt: &str, default: Option<&str>) -> Result<Option<String>> {
    let mut builder = Text::new(prompt).with_render_config(minimal_render_config());

    if let Some(d) = default {
        if !d.is_empty() {
            builder = builder.with_default(d);
        }
    }

    let result = builder.prompt_skippable()?;
    Ok(result)
}

/// Prompt for yes/no confirmation (default: no)
pub fn confirm(prompt: &str) -> Result<bool> {
    let result = Confirm::new(prompt)
        .with_render_config(minimal_render_config())
        .with_default(false)
        .prompt()?;
    Ok(result)
}

/// Print a simple status message
pub fn status(message: &str) {
    println!("{}", message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("Error: {}", message);
}

/// Format a person for selection display: "Name (email)"
fn format_person_for_select(
    person: &crate::models::Person,
    display_info: &std::collections::HashMap<uuid::Uuid, (Option<String>, Option<String>)>,
) -> String {
    let name = person.display_name.as_deref().unwrap_or("(unnamed)");
    let email = display_info
        .get(&person.id)
        .and_then(|(e, _)| e.clone());

    match email {
        Some(e) => format!("{} ({})", name, e),
        None => name.to_string(),
    }
}

/// Display a contact selection menu using inquire Select
/// Returns the selected Person or None if cancelled
pub fn select_contact(
    db: &crate::db::Database,
    persons: &[crate::models::Person],
) -> Result<Option<crate::models::Person>> {
    if persons.is_empty() {
        return Ok(None);
    }

    // Single match goes directly through (no selection needed)
    if persons.len() == 1 {
        return Ok(Some(persons[0].clone()));
    }

    // Get display info for all persons
    let person_ids: Vec<_> = persons.iter().map(|p| p.id).collect();
    let display_info = db.get_display_info_for_persons(&person_ids)?;

    // Build display strings
    let options: Vec<String> = persons
        .iter()
        .map(|p| format_person_for_select(p, &display_info))
        .collect();

    let result = Select::new("Select:", options.clone())
        .with_render_config(minimal_render_config())
        .with_page_size(visible_lines())
        .with_vim_mode(true)
        .prompt_skippable()?;

    match result {
        Some(selected) => {
            // Find which person was selected
            let idx = options.iter().position(|o| *o == selected).unwrap_or(0);
            Ok(Some(persons[idx].clone()))
        }
        None => Ok(None),
    }
}

// ============================================================================
// Form Input Helpers
// ============================================================================

/// Result type for form inputs that can be cancelled
pub enum FormResult<T> {
    Value(T),
    Cancelled,
}

/// Prompt for a field with optional current value
/// Format: `field [current]: ` or `field: ` if no current value
/// Returns None if cancelled (Ctrl+C/Escape), Some(value) otherwise
/// Empty input returns the current value (or empty string if no current)
pub fn prompt_field(field: &str, current: Option<&str>) -> Result<FormResult<String>> {
    let prompt = match current {
        Some(val) if !val.is_empty() => format!("{} [{}]: ", field, truncate_for_display(val, 30)),
        _ => format!("{}: ", field),
    };

    let result = Text::new(&prompt)
        .with_render_config(minimal_render_config())
        .prompt();

    match result {
        Ok(input) => {
            let input = input.trim();
            if input.is_empty() {
                // Keep current value
                Ok(FormResult::Value(current.unwrap_or("").to_string()))
            } else {
                Ok(FormResult::Value(input.to_string()))
            }
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            Ok(FormResult::Cancelled)
        }
        Err(e) => Err(e.into()),
    }
}

/// Prompt for an optional field (returns empty string if skipped)
pub fn prompt_field_optional(field: &str) -> Result<FormResult<String>> {
    let prompt = format!("{}: ", field);

    let result = Text::new(&prompt)
        .with_render_config(minimal_render_config())
        .prompt();

    match result {
        Ok(input) => Ok(FormResult::Value(input.trim().to_string())),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            Ok(FormResult::Cancelled)
        }
        Err(e) => Err(e.into()),
    }
}

/// Validate email format
pub fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

/// Truncate string for display in prompts
fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_render_config() {
        let config = minimal_render_config();
        // Just verify it doesn't panic
        let _ = config;
    }
}
