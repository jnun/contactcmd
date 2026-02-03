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
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use inquire::{ui::RenderConfig, Confirm, InquireError, Select, Text};
use std::io::{self, Write};

// ============================================================================
// Terminal Writer - Universal Design System
// ============================================================================

/// Newline constant for raw mode (carriage return + line feed)
pub const RAW_NEWLINE: &str = "\r\n";

/// Terminal writer that handles raw mode automatically.
///
/// In raw mode, newlines are `\r\n` (carriage return + line feed).
/// In cooked mode, newlines are `\n`.
///
/// This struct manages the mode and provides consistent output methods.
/// Designed for antifragility: terminal errors are logged but don't crash.
pub struct Term {
    raw_mode: bool,
    stdout: io::Stdout,
}

impl Term {
    /// Create a new terminal writer in cooked (normal) mode
    #[inline]
    pub fn new() -> Self {
        Self {
            raw_mode: false,
            stdout: io::stdout(),
        }
    }

    /// Create a new terminal writer and enter raw mode.
    /// On failure, returns cooked mode Term (graceful degradation).
    pub fn raw() -> Self {
        match enable_raw_mode() {
            Ok(()) => Self {
                raw_mode: true,
                stdout: io::stdout(),
            },
            Err(_) => {
                // Graceful degradation: return cooked mode
                Self::new()
            }
        }
    }

    /// Create a new terminal writer and enter raw mode, returning error on failure.
    pub fn try_raw() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self {
            raw_mode: true,
            stdout: io::stdout(),
        })
    }

    /// Check if terminal is in raw mode
    #[inline]
    pub fn is_raw(&self) -> bool {
        self.raw_mode
    }

    /// Get the appropriate newline for current mode
    #[inline]
    pub fn newline(&self) -> &'static str {
        if self.raw_mode { "\r\n" } else { "\n" }
    }

    /// Write a line with proper newline handling
    #[inline]
    pub fn line(&mut self, s: &str) {
        if self.raw_mode {
            let _ = write!(self.stdout, "{}\r\n", s);
        } else {
            let _ = writeln!(self.stdout, "{}", s);
        }
    }

    /// Write multiple lines
    pub fn lines(&mut self, lines: &[&str]) {
        for line in lines {
            self.line(line);
        }
    }

    /// Write text without a newline
    #[inline]
    pub fn print(&mut self, s: &str) {
        let _ = write!(self.stdout, "{}", s);
    }

    /// Clear the screen and move cursor to top-left
    pub fn clear(&mut self) {
        let _ = self.stdout.execute(Clear(ClearType::All));
        let _ = self.stdout.execute(cursor::MoveTo(0, 0));
        let _ = self.stdout.flush();
    }

    /// Flush output buffer
    #[inline]
    pub fn flush(&mut self) {
        let _ = self.stdout.flush();
    }

    /// Move cursor to position
    #[inline]
    pub fn move_to(&mut self, col: u16, row: u16) {
        let _ = self.stdout.execute(cursor::MoveTo(col, row));
    }

    /// Clear from cursor to end of screen
    #[inline]
    pub fn clear_below(&mut self) {
        let _ = self.stdout.execute(Clear(ClearType::FromCursorDown));
    }
}

impl Default for Term {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        if self.raw_mode {
            let _ = disable_raw_mode();
        }
    }
}

// ============================================================================
// Status Bar Builder
// ============================================================================

/// Maximum actions a status bar can hold (stack-allocated)
const MAX_STATUS_ACTIONS: usize = 8;

/// Builder for consistent status bar formatting.
///
/// Example output: "12/345  [e]dit [m]essages [q]uit"
///
/// Uses a fixed-size array to avoid heap allocation for typical use cases.
pub struct StatusBar<'a> {
    counter: Option<(usize, usize)>,
    actions: [Option<(&'a str, &'a str)>; MAX_STATUS_ACTIONS],
    action_count: usize,
}

impl<'a> StatusBar<'a> {
    /// Create a new empty status bar
    #[inline]
    pub fn new() -> Self {
        Self {
            counter: None,
            actions: [None; MAX_STATUS_ACTIONS],
            action_count: 0,
        }
    }

    /// Add a counter (current/total)
    #[inline]
    pub fn counter(mut self, current: usize, total: usize) -> Self {
        self.counter = Some((current, total));
        self
    }

    /// Add an action hint (key, label)
    /// Example: `.action("e", "dit")` produces `[e]dit`
    #[inline]
    pub fn action(mut self, key: &'a str, label: &'a str) -> Self {
        if self.action_count < MAX_STATUS_ACTIONS {
            self.actions[self.action_count] = Some((key, label));
            self.action_count += 1;
        }
        self
    }

    /// Add a visual separator (" | ")
    #[inline]
    pub fn separator(mut self) -> Self {
        if self.action_count < MAX_STATUS_ACTIONS {
            self.actions[self.action_count] = Some(("|", ""));
            self.action_count += 1;
        }
        self
    }

    /// Render the status bar to a string.
    /// Auto-switches to compact two-line format for narrow terminals.
    pub fn render(&self) -> String {
        let full = self.render_full();
        let width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

        if full.len() > width.saturating_sub(5) {
            self.render_compact()
        } else {
            full
        }
    }

    fn render_full(&self) -> String {
        let estimated_cap = 12 + (self.action_count * 18);
        let mut result = String::with_capacity(estimated_cap);

        if let Some((current, total)) = self.counter {
            use std::fmt::Write;
            let _ = write!(result, "{}/{}", current, total);
        }

        for i in 0..self.action_count {
            if let Some((key, label)) = self.actions[i] {
                if key == "|" {
                    result.push_str(" | ");
                } else {
                    if !result.is_empty() && !result.ends_with(" | ") {
                        result.push(' ');
                    }
                    result.push('[');
                    result.push_str(key);
                    result.push(']');
                    result.push_str(label);
                }
            }
        }

        result
    }

    fn render_compact(&self) -> String {
        let mut line1 = String::new();
        let mut line2 = String::new();

        if let Some((current, total)) = self.counter {
            use std::fmt::Write;
            let _ = write!(line1, "{}/{}", current, total);
        }

        for i in 0..self.action_count {
            if let Some((key, label)) = self.actions[i] {
                if key == "|" {
                    line2.push_str(" | ");
                } else if label.is_empty() {
                    if !line2.is_empty() && !line2.ends_with(" | ") {
                        line2.push(' ');
                    }
                    line2.push('[');
                    line2.push_str(key);
                    line2.push(']');
                } else {
                    if !line2.is_empty() && !line2.ends_with(" | ") {
                        line2.push(' ');
                    }
                    line2.push('[');
                    line2.push_str(key);
                    line2.push(']');
                    line2.push_str(label);
                }
            }
        }

        format!("{}\n{}", line1, line2.trim())
    }
}

impl Default for StatusBar<'_> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Layout Primitives
// ============================================================================

/// Return selection prefix for list items
#[inline]
pub fn selection_prefix(selected: bool) -> &'static str {
    if selected { "> " } else { "  " }
}

/// Truncate a string to max_chars, adding ellipsis if needed.
/// Uses single-pass iteration for efficiency.
/// Result will be at most max_chars characters (including ellipsis if truncated).
pub fn truncate(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let truncate_at = max_chars.saturating_sub(1); // Leave room for ellipsis
    let mut char_count = 0;
    let mut truncate_idx = 0;
    let mut total_chars = 0;

    for (idx, _) in s.char_indices() {
        if char_count == truncate_at {
            truncate_idx = idx;
        }
        char_count += 1;
        total_chars += 1;
        if total_chars > max_chars {
            // String exceeds max, truncate at truncate_at position
            return format!("{}…", &s[..truncate_idx]);
        }
    }

    // String fits within max_chars, no truncation needed
    s.to_string()
}

/// Format a counter string (e.g., "12/345")
#[inline]
pub fn counter(current: usize, total: usize) -> String {
    format!("{}/{}", current, total)
}

/// Format task action label for status bar (e.g., "ask" or "ask (3)")
#[inline]
pub fn task_action_label(pending_count: u32) -> String {
    if pending_count > 0 {
        format!("ask ({})", pending_count)
    } else {
        "ask".to_string()
    }
}

// ============================================================================
// Message Functions
// ============================================================================

/// Print a status message to stdout
#[inline]
pub fn status(msg: &str) {
    println!("{}", msg);
}

/// Print an error message to stderr
#[inline]
pub fn error(msg: &str) {
    eprintln!("Error: {}", msg);
}

/// Print a warning message to stderr
#[inline]
pub fn warning(msg: &str) {
    eprintln!("Warning: {}", msg);
}

// ============================================================================
// Raw Mode Guard (legacy compatibility)
// ============================================================================

/// RAII guard that ensures raw mode is disabled on drop
pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Default for RawModeGuard {
    fn default() -> Self {
        // This will panic if raw mode fails, which is intentional for Default
        Self::new().expect("Failed to enable raw mode")
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Wait for any key press, accepting Enter, q, or Esc
pub fn wait_for_key() -> Result<()> {
    let _guard = RawModeGuard::new()?;
    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            if matches!(code, KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc) {
                break;
            }
        }
    }
    Ok(())
}

/// Display context-sensitive help screen
pub fn show_help(context: &str) -> Result<()> {
    clear_screen()?;

    let help_text = match context {
        "browse" | "search" => r#"
NAVIGATION

  j / ↓ / →     Next contact
  k / ↑ / ←     Previous contact
  g / G         Jump to first / last
  v             Toggle card/table view

ACTIONS

  e             Edit contact (all fields)
  m             View messages
  n             Edit notes
  t             View/manage tasks
  d             Delete (with confirmation)

EXIT

  q / Esc       Back to previous screen
  Q             Quit application
  ?             This help screen
"#,
        "list" => r#"
LIST NAVIGATION

  j / ↓         Move down
  k / ↑         Move up
  g / Home      Jump to first
  G / End       Jump to last
  Enter         View contact details

EXIT

  q / Esc       Back to menu
  Q             Quit application
  ?             This help screen
"#,
        "tasks" => r#"
TASKS

  j / ↓         Move down
  k / ↑         Move up
  n             New task
  c             Toggle complete
  d             Delete task
  h             Hide/show completed
  s             Toggle sort mode

EXIT

  q / Esc       Back
  Q             Quit application
  ?             This help screen
"#,
        _ => "Press any key to return.",
    };

    println!("{}", help_text);
    println!("\nPress any key to return...");

    let _guard = RawModeGuard::new()?;
    let _ = event::read()?;
    Ok(())
}

/// Multi-line input with vim-style modal editing
/// - Edit mode: type normally, Escape enters command mode
/// - Command mode: action key to confirm, 'q' to quit, 'r' to return to edit
/// Returns None if cancelled, Some(text) if confirmed
///
/// action_label: e.g. "send" or "keep" - first char is the hotkey
pub fn multiline_input_raw(prompt: &str, action_label: &str) -> Result<Option<String>> {
    use crossterm::event::KeyModifiers;

    println!("{}", prompt);
    println!();

    let mut stdout = io::stdout();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut command_mode = false;

    let action_char = action_label.chars().next().unwrap_or('s').to_ascii_lowercase();
    // Format: [s]end or [k]eep
    let action_display = format!("[{}]{}", action_char, &action_label[1..]);

    {
        let _guard = RawModeGuard::new()?;

        loop {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                if command_mode {
                    // Command mode: single key commands
                    match code {
                        KeyCode::Char(c) if c.to_ascii_lowercase() == action_char => {
                            // Confirm action
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            break;
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            // Quit/Cancel
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(None);
                        }
                        KeyCode::Esc | KeyCode::Char('r') | KeyCode::Char('R') => {
                            // Return to edit mode
                            command_mode = false;
                            // Clear command menu (2 lines: blank + commands) and restore cursor
                            write!(stdout, "\x1b[2A\r\x1b[J")?; // Move up 2, clear to end
                            // Reprint current line
                            write!(stdout, "{}", current_line)?;
                            stdout.flush()?;
                        }
                        _ => {
                            // Ignore other keys in command mode
                        }
                    }
                } else {
                    // Edit mode: normal text input
                    match code {
                        KeyCode::Esc => {
                            // Enter command mode
                            command_mode = true;
                            write!(stdout, "\r\n\n{} [q]uit [r]eturn: ", action_display)?;
                            stdout.flush()?;
                        }
                        KeyCode::Enter => {
                            // Add current line and start new one
                            lines.push(current_line.clone());
                            current_line.clear();
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                        }
                        KeyCode::Backspace => {
                            if !current_line.is_empty() {
                                // Remove character from current line
                                current_line.pop();
                                write!(stdout, "\x08 \x08")?;
                                stdout.flush()?;
                            } else if !lines.is_empty() {
                                // Current line is empty - go back to previous line
                                current_line = lines.pop().unwrap();
                                // Move cursor up one line, then to end of that line
                                // \x1b[A = move up, \r = go to start, then print the line
                                write!(stdout, "\x1b[A\r\x1b[K{}", current_line)?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Char(c) => {
                            // Handle Ctrl+C
                            if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                write!(stdout, "\r\n")?;
                                stdout.flush()?;
                                return Ok(None);
                            }
                            current_line.push(c);
                            write!(stdout, "{}", c)?;
                            stdout.flush()?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Include any remaining content on current line
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Remove trailing empty lines
    while lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    if lines.is_empty() {
        return Ok(None);
    }

    Ok(Some(lines.join("\n")))
}

/// Multi-line input for email compose with signature
pub fn multiline_input_email(signature_content: &str) -> Result<Option<String>> {
    // Show signature preview
    if !signature_content.trim().is_empty() {
        println!("┌─ signature ─────────────────┐");
        for line in signature_content.lines() {
            println!("│ {}", line);
        }
        println!("└─────────────────────────────┘");
        println!();
    }

    let body = multiline_input_raw(
        "Message: ([esc] for commands)",
        "send"
    )?;

    let Some(message) = body else {
        return Ok(None);
    };

    // Append signature if provided
    let result = if signature_content.trim().is_empty() {
        message
    } else {
        format!("{}\n\n{}", message, signature_content.trim())
    };

    Ok(Some(result))
}

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

/// Prompt for undo with timeout. Returns true if user pressed 'u'.
pub fn prompt_undo(message: &str, timeout_secs: u64) -> Result<bool> {
    use std::time::Duration;

    print!("{} - press [u] to undo ({}s)", message, timeout_secs);
    io::stdout().flush()?;

    let _guard = RawModeGuard::new()?;

    if crossterm::event::poll(Duration::from_secs(timeout_secs))? {
        if let Event::Key(KeyEvent { code: KeyCode::Char('u'), .. }) = event::read()? {
            println!();
            return Ok(true);
        }
    }

    println!();
    Ok(false)
}

/// Format a person for selection display: "Name (email)"
fn format_person_for_select(
    person: &crate::models::Person,
    display_info: &std::collections::HashMap<uuid::Uuid, (Option<String>, Option<String>)>,
) -> String {
    let name = get_display_name(person);
    let email = display_info
        .get(&person.id)
        .and_then(|(e, _)| e.clone());

    match email {
        Some(e) => format!("{} ({})", name, e),
        None => name,
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
// Person Lookup Helpers
// ============================================================================

/// Get a display name for a person, with fallbacks if display_name is empty.
/// Falls back to: given+family -> given -> family -> nickname -> preferred -> "(unnamed)"
pub fn get_display_name(person: &crate::models::Person) -> String {
    // Try display_name first
    if let Some(ref name) = person.display_name {
        if !name.is_empty() {
            return name.clone();
        }
    }
    // Try constructing from given + family
    match (&person.name_given, &person.name_family) {
        (Some(g), Some(f)) if !g.is_empty() && !f.is_empty() => format!("{} {}", g, f),
        (Some(g), _) if !g.is_empty() => g.clone(),
        (_, Some(f)) if !f.is_empty() => f.clone(),
        _ => person.name_nickname.clone()
            .filter(|s| !s.is_empty())
            .or_else(|| person.preferred_name.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "(unnamed)".to_string()),
    }
}

/// Find a person by UUID or name search.
/// - If identifier is a valid UUID, looks up directly
/// - Otherwise, searches by name and prompts for selection if multiple matches
/// Returns None if not found or selection cancelled.
pub fn find_person_by_identifier(
    db: &crate::db::Database,
    identifier: &str,
) -> Result<Option<crate::models::Person>> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Ok(None);
    }

    // Try parsing as UUID first
    if let Ok(uuid) = uuid::Uuid::parse_str(identifier) {
        return db.get_person_by_id(uuid).map_err(Into::into);
    }

    // Search by name
    let words: Vec<&str> = identifier.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, u32::MAX)?;

    match results.len() {
        0 => Ok(None),
        1 => Ok(Some(results.into_iter().next().unwrap())),
        _ => select_contact(db, &results),
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
    let has_value = current.map(|v| !v.is_empty()).unwrap_or(false);
    let prompt = match current {
        Some(val) if !val.is_empty() => format!("{} [{}] (- clears): ", field, truncate_for_display(val, 30)),
        _ => format!("{}: ", field),
    };

    let result = Text::new(&prompt)
        .with_render_config(minimal_render_config())
        .prompt();

    match result {
        Ok(input) => {
            let input = input.trim();
            if input == "-" && has_value {
                // Clear the field
                Ok(FormResult::Value(String::new()))
            } else if input.is_empty() {
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

// ============================================================================
// Combined Search Input
// ============================================================================

/// Search field options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchField {
    All,
    Name,
    City,
    State,
    Tag,
}

impl SearchField {
    pub fn all() -> &'static [SearchField] {
        &[
            SearchField::All,
            SearchField::Name,
            SearchField::City,
            SearchField::State,
            SearchField::Tag,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            SearchField::All => "All fields",
            SearchField::Name => "Name",
            SearchField::City => "City",
            SearchField::State => "State",
            SearchField::Tag => "Tag",
        }
    }

    pub fn to_field_str(self) -> Option<&'static str> {
        match self {
            SearchField::All => None,
            SearchField::Name => Some("name"),
            SearchField::City => Some("city"),
            SearchField::State => Some("state"),
            SearchField::Tag => None, // Tag is handled specially
        }
    }

    pub fn is_tag(self) -> bool {
        matches!(self, SearchField::Tag)
    }
}

/// Result of the combined search input
pub struct SearchInput {
    pub query: String,
    pub field: SearchField,
}

/// Combined search input: text entry + field selector on same screen
/// Arrow up/down changes field, typing enters text, Enter submits
/// Returns None if cancelled (Esc)
pub fn search_input_combined(prompt: &str) -> Result<Option<SearchInput>> {
    use crossterm::event::KeyModifiers;

    let fields = SearchField::all();
    let mut selected_idx: usize = 0;
    let mut query = String::new();
    let mut stdout = io::stdout();

    // Helper to render the current state
    let render = |stdout: &mut io::Stdout, query: &str, selected_idx: usize| -> Result<()> {
        // Move to start and clear screen from cursor
        stdout.execute(cursor::MoveTo(0, 0))?;
        stdout.execute(Clear(ClearType::FromCursorDown))?;

        // Print prompt and query (use \r\n in raw mode)
        write!(stdout, "{}{}\r\n\r\n", prompt, query)?;

        // Print field options
        for (i, field) in fields.iter().enumerate() {
            if i == selected_idx {
                write!(stdout, "> {}\r\n", field.label())?;
            } else {
                write!(stdout, "  {}\r\n", field.label())?;
            }
        }

        // Move cursor back to end of query line
        let query_col = (prompt.len() + query.len()) as u16;
        stdout.execute(cursor::MoveTo(query_col, 0))?;
        stdout.flush()?;
        Ok(())
    };

    {
        let _guard = RawModeGuard::new()?;

        // Initial render
        render(&mut stdout, &query, selected_idx)?;

        loop {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => {
                        // Submit
                        write!(stdout, "\r\n")?;
                        stdout.flush()?;
                        break;
                    }
                    KeyCode::Esc => {
                        // Cancel
                        write!(stdout, "\r\n")?;
                        stdout.flush()?;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        // Move selection up (arrows only - j/k reserved for typing)
                        if selected_idx > 0 {
                            selected_idx -= 1;
                        }
                        render(&mut stdout, &query, selected_idx)?;
                    }
                    KeyCode::Down => {
                        // Move selection down (arrows only - j/k reserved for typing)
                        if selected_idx < fields.len() - 1 {
                            selected_idx += 1;
                        }
                        render(&mut stdout, &query, selected_idx)?;
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        render(&mut stdout, &query, selected_idx)?;
                    }
                    KeyCode::Char(c) => {
                        // Handle Ctrl+C
                        if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(None);
                        }
                        query.push(c);
                        render(&mut stdout, &query, selected_idx)?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(Some(SearchInput {
        query,
        field: fields[selected_idx],
    }))
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

    #[test]
    fn test_search_field_labels() {
        assert_eq!(SearchField::All.label(), "All fields");
        assert_eq!(SearchField::Name.label(), "Name");
        assert_eq!(SearchField::City.label(), "City");
        assert_eq!(SearchField::Tag.label(), "Tag");
    }

    #[test]
    fn test_search_field_to_str() {
        assert_eq!(SearchField::All.to_field_str(), None);
        assert_eq!(SearchField::Name.to_field_str(), Some("name"));
        assert_eq!(SearchField::City.to_field_str(), Some("city"));
    }

    // ============================================================================
    // Design System Tests
    // ============================================================================

    #[test]
    fn test_status_bar_empty() {
        let bar = StatusBar::new();
        assert_eq!(bar.render(), "");
    }

    #[test]
    fn test_status_bar_counter_only() {
        let bar = StatusBar::new().counter(5, 10);
        assert_eq!(bar.render(), "5/10");
    }

    #[test]
    fn test_status_bar_actions_only() {
        let bar = StatusBar::new()
            .action("e", "dit")
            .action("q", "uit");
        assert_eq!(bar.render(), "[e]dit [q]uit");
    }

    #[test]
    fn test_status_bar_full() {
        let bar = StatusBar::new()
            .counter(3, 15)
            .action("e", "dit")
            .action("m", "essages")
            .action("q", "uit");
        assert_eq!(bar.render(), "3/15 [e]dit [m]essages [q]uit");
    }

    #[test]
    fn test_status_bar_with_separator() {
        let bar = StatusBar::new()
            .counter(1, 10)
            .action("e", "dit")
            .action("d", "el")
            .separator()
            .action("?", "")
            .action("q", "");
        assert_eq!(bar.render(), "1/10 [e]dit [d]el | [?] [q]");
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn test_truncate_unicode() {
        // "日本語" is 3 characters
        assert_eq!(truncate("日本語テスト", 4), "日本語…");
    }

    #[test]
    fn test_counter_format() {
        assert_eq!(counter(1, 100), "1/100");
        assert_eq!(counter(50, 50), "50/50");
    }

    #[test]
    fn test_task_action_label() {
        assert_eq!(task_action_label(0), "ask");
        assert_eq!(task_action_label(1), "ask (1)");
        assert_eq!(task_action_label(5), "ask (5)");
        assert_eq!(task_action_label(999), "ask (999)");
    }

    #[test]
    fn test_selection_prefix() {
        assert_eq!(selection_prefix(true), "> ");
        assert_eq!(selection_prefix(false), "  ");
    }

    #[test]
    fn test_term_cooked_mode() {
        let term = Term::new();
        assert!(!term.is_raw());
    }
}
