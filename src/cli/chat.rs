//! Chat interface for contactcmd
//!
//! A DOS-style command interface for interacting with contactcmd.
//! Supports both traditional commands and conversational AI (when configured).

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::{self, Write};

use crate::cli::ai::{AiChatSession, CommandFeedback};
use crate::cli::list::{run_browse, ViewMode};
use crate::cli::ui::{clear_screen, RawModeGuard};
use crate::db::Database;
use crate::models::Person;

/// Chat command parsed from user input
#[derive(Debug, Clone, PartialEq)]
pub enum ChatCommand {
    Help,
    Search(String),
    Browse,
    List,
    Add,
    Import,
    Sync,
    Messages(String),
    Recent(u32),
    Bridge,
    Setup,
    Learn,
    Teach(String),
    Clear,
    Quit,
    Unknown(String),
    Empty,
}

impl ChatCommand {
    /// Parse a command string into a ChatCommand
    ///
    /// Commands must start with `/` (e.g., `/search`, `/i`).
    /// Everything else is treated as chat for the AI.
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input.is_empty() {
            return ChatCommand::Empty;
        }

        // Handle common commands without / prefix
        let lower = input.to_lowercase();
        if matches!(lower.as_str(), "q" | "quit" | "exit" | "bye") {
            return ChatCommand::Quit;
        }
        if matches!(lower.as_str(), "help" | "?") {
            return ChatCommand::Help;
        }

        // Commands with / prefix
        if !input.starts_with('/') {
            return ChatCommand::Unknown(input.to_string());
        }

        // Strip the leading / and parse
        let input = &input[1..];
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

        match cmd.as_str() {
            "h" | "help" | "?" => ChatCommand::Help,
            "s" | "search" | "find" => ChatCommand::Search(args),
            "b" | "browse" => ChatCommand::Browse,
            "l" | "list" | "ls" => ChatCommand::List,
            "a" | "add" | "new" => ChatCommand::Add,
            "i" | "import" => ChatCommand::Import,
            "sync" => ChatCommand::Sync,
            "m" | "msg" | "messages" => ChatCommand::Messages(args),
            "r" | "recent" => {
                // Parse optional days argument, default to 7
                let days = if args.is_empty() {
                    7
                } else {
                    args.parse().unwrap_or(7)
                };
                ChatCommand::Recent(days)
            }
            "bridge" => ChatCommand::Bridge,
            "setup" => ChatCommand::Setup,
            "learn" => ChatCommand::Learn,
            "teach" => ChatCommand::Teach(args),
            "clear" | "cls" => ChatCommand::Clear,
            "q" | "quit" | "exit" => ChatCommand::Quit,
            _ => ChatCommand::Unknown(format!("/{}", input)),
        }
    }
}

/// Chat session state
pub struct ChatSession<'a> {
    db: &'a Database,
    history: Vec<String>,
    history_idx: Option<usize>,
    ai_session: Option<AiChatSession>,
    /// Last search results for /browse command
    last_results: Vec<Person>,
}

impl<'a> ChatSession<'a> {
    pub fn new(db: &'a Database) -> Self {
        // Try to load AI session from database config
        let ai_session = AiChatSession::from_database(db).ok().flatten();

        Self {
            db,
            history: Vec::new(),
            history_idx: None,
            ai_session,
            last_results: Vec::new(),
        }
    }

    /// Check if AI is available
    fn has_ai(&self) -> bool {
        self.ai_session.as_ref().map(|s| s.is_ready()).unwrap_or(false)
    }

    /// Run the chat interface
    pub fn run(&mut self) -> Result<bool> {
        clear_screen()?;
        self.print_welcome();

        loop {
            let input = self.read_command()?;

            match input {
                Some(cmd_str) => {
                    if !cmd_str.is_empty() {
                        self.history.push(cmd_str.clone());
                    }
                    self.history_idx = None;

                    let cmd = ChatCommand::parse(&cmd_str);
                    match self.execute(cmd)? {
                        ChatAction::Continue => {}
                        ChatAction::Quit => return Ok(true),
                        ChatAction::Back => return Ok(false),
                    }
                }
                None => {
                    // User pressed Escape
                    return Ok(false);
                }
            }
        }
    }

    fn print_welcome(&self) {
        println!("contactcmd chat");
        println!("───────────────");
        println!();
        if self.has_ai() {
            println!("AI is enabled. Type naturally or use /commands.");
            println!("Type /help for commands, /quit to exit.");
        } else {
            println!("Type /help for commands, /quit to exit.");
            println!("(Run /setup to enable AI features)");
        }
        println!();

        // Show "Learn something?" suggestion
        self.show_learn_suggestion();
    }

    /// Show a subtle suggestion to learn a feature
    fn show_learn_suggestion(&self) {
        match self.db.all_features_learned_once() {
            Ok(all_learned) => {
                if all_learned {
                    // Refresher mode
                    if let Ok((learned, total)) = self.db.get_learning_stats() {
                        println!("Tip: /learn to refresh your memory on a feature ({}/{})", learned, total);
                    }
                } else {
                    // First-time learning
                    if let Ok((learned, total)) = self.db.get_learning_stats() {
                        println!("Tip: /learn to discover a feature ({}/{} learned)", learned, total);
                    }
                }
                println!();
            }
            Err(_) => {} // Silently skip if there's an error
        }
    }

    fn print_prompt(&self) {
        print!("> ");
        let _ = io::stdout().flush();
    }

    /// Read a command with line editing and history
    fn read_command(&mut self) -> Result<Option<String>> {
        self.print_prompt();

        let mut input = String::new();
        let mut cursor_pos = 0;
        let mut stdout = io::stdout();

        {
            let _guard = RawModeGuard::new()?;

            loop {
                if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                    match code {
                        KeyCode::Enter => {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(Some(input));
                        }
                        KeyCode::Esc => {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(None);
                        }
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(None);
                        }
                        KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Clear screen
                            clear_screen()?;
                            self.print_welcome();
                            self.print_prompt();
                            write!(stdout, "{}", input)?;
                            stdout.flush()?;
                        }
                        KeyCode::Up => {
                            // History navigation
                            if !self.history.is_empty() {
                                let new_idx = match self.history_idx {
                                    None => self.history.len() - 1,
                                    Some(0) => 0,
                                    Some(i) => i - 1,
                                };
                                self.history_idx = Some(new_idx);
                                input = self.history[new_idx].clone();
                                cursor_pos = input.len();
                                // Clear line and reprint
                                write!(stdout, "\r> \x1b[K{}", input)?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Down => {
                            // History navigation
                            if let Some(idx) = self.history_idx {
                                if idx + 1 < self.history.len() {
                                    self.history_idx = Some(idx + 1);
                                    input = self.history[idx + 1].clone();
                                } else {
                                    self.history_idx = None;
                                    input.clear();
                                }
                                cursor_pos = input.len();
                                write!(stdout, "\r> \x1b[K{}", input)?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Left => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                write!(stdout, "\x1b[D")?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Right => {
                            if cursor_pos < input.len() {
                                cursor_pos += 1;
                                write!(stdout, "\x1b[C")?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Home => {
                            cursor_pos = 0;
                            write!(stdout, "\r> ")?;
                            stdout.flush()?;
                        }
                        KeyCode::End => {
                            cursor_pos = input.len();
                            write!(stdout, "\r> {}", input)?;
                            stdout.flush()?;
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                input.remove(cursor_pos);
                                // Reprint line from cursor
                                let remaining = &input[cursor_pos..];
                                write!(stdout, "\x08{} \x1b[{}D", remaining, remaining.len() + 1)?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Delete => {
                            if cursor_pos < input.len() {
                                input.remove(cursor_pos);
                                let remaining = &input[cursor_pos..];
                                write!(stdout, "{} \x1b[{}D", remaining, remaining.len() + 1)?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Char(c) => {
                            input.insert(cursor_pos, c);
                            cursor_pos += 1;
                            // Print character and rest of line
                            let remaining = &input[cursor_pos..];
                            if remaining.is_empty() {
                                write!(stdout, "{}", c)?;
                            } else {
                                write!(stdout, "{}{}\x1b[{}D", c, remaining, remaining.len())?;
                            }
                            stdout.flush()?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Execute a chat command
    fn execute(&mut self, cmd: ChatCommand) -> Result<ChatAction> {
        match cmd {
            ChatCommand::Help => {
                self.show_help();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Search(query) => {
                if query.is_empty() {
                    println!("Usage: /search <query>");
                    println!("       /s john");
                } else {
                    self.do_search_with_prompt(&query)?;
                }
                Ok(ChatAction::Continue)
            }
            ChatCommand::Browse => {
                if self.last_results.is_empty() {
                    println!("No results to browse. Use /search first.");
                } else {
                    let count = self.last_results.len();
                    println!("Browsing {} contact{}...\n", count, if count == 1 { "" } else { "s" });
                    run_browse(self.db, self.last_results.clone(), ViewMode::Card)?;
                    clear_screen()?;
                    self.print_welcome();
                }
                Ok(ChatAction::Continue)
            }
            ChatCommand::List => {
                self.do_list_with_prompt()?;
                Ok(ChatAction::Continue)
            }
            ChatCommand::Add => {
                println!("Adding new contact...");
                // TODO: Implement add flow
                println!("(Add contact form will appear here)");
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Import => {
                println!("Import contacts from CSV...");
                // TODO: Implement import
                println!("(Import wizard will appear here)");
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Sync => {
                println!("Syncing contacts...");
                // TODO: Implement sync
                println!("(Sync status will appear here)");
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Messages(query) => {
                if query.is_empty() {
                    println!("Usage: messages <query>");
                    println!("       m hello");
                } else {
                    println!("Searching messages for '{}'...", query);
                    // TODO: Implement message search
                    println!("(Message results will appear here)");
                }
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Recent(days) => {
                self.do_recent(days)?;
                Ok(ChatAction::Continue)
            }
            ChatCommand::Bridge => {
                println!("Bridge status...");
                // TODO: Show bridge status
                println!("(Bridge status will appear here)");
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Setup => {
                println!("Opening setup...");
                // TODO: Launch setup
                println!("(Setup wizard will appear here)");
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Learn => {
                self.do_learn()?;
                Ok(ChatAction::Continue)
            }
            ChatCommand::Teach(topic) => {
                if topic.is_empty() {
                    println!("Usage: /teach <topic>");
                    println!("       /teach search");
                    println!("       /teach messages");
                } else {
                    self.do_teach(&topic)?;
                }
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Clear => {
                clear_screen()?;
                self.print_welcome();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Quit => {
                Ok(ChatAction::Quit)
            }
            ChatCommand::Unknown(input) => {
                // If AI is available, route unknown input to AI
                if self.has_ai() {
                    self.handle_ai_message(&input)?;
                } else {
                    println!("Unknown command: {}", input);
                    println!("Type 'help' for available commands.");
                }
                println!();
                Ok(ChatAction::Continue)
            }
            ChatCommand::Empty => {
                Ok(ChatAction::Continue)
            }
        }
    }

    fn show_help(&self) {
        println!();
        println!("COMMANDS (prefix with /)");
        println!("────────────────────────");
        println!();
        println!("  /[h]elp              Show this help");
        println!("  /[s]earch <query>    Search contacts");
        println!("  /[b]rowse            Browse last results in TUI");
        println!("  /[l]ist              List all contacts");
        println!("  /[a]dd               Add new contact");
        println!("  /[i]mport            Import from CSV");
        println!("  /[m]essages <query>  Search messages");
        println!("  /[r]ecent [days]     Recent SMS/iMessage contacts");
        println!("  /sync                Sync with Mac contacts");
        println!("  /bridge              Show bridge status");
        println!("  /setup               Configure settings");
        println!("  /learn               Learn a new feature");
        println!("  /teach <topic>       Get help on a topic");
        println!("  /clear               Clear screen");
        println!("  /[q]uit              Exit chat");
        println!();
        if self.has_ai() {
            println!("CHAT");
            println!("────");
            println!();
            println!("  Type naturally to interact with your contacts:");
            println!("  \"find contacts at Google\"");
            println!("  \"show me Alice's details\"");
            println!("  \"when did I last message Bob?\"");
            println!();
        }
        println!("NAVIGATION");
        println!("──────────");
        println!();
        println!("  Up/Down             Command history");
        println!("  Ctrl+L              Clear screen");
        println!("  Ctrl+C / Esc        Cancel / go back");
        println!();
    }

    /// Handle a message through the AI
    ///
    /// IMPORTANT: The AI has NO access to user data. It only suggests commands.
    /// Commands are captured directly from tool execution (not parsed from AI text).
    fn handle_ai_message(&mut self, message: &str) -> Result<()> {
        if let Some(ref mut ai) = self.ai_session {
            // Show thinking indicator
            print!("Thinking...");
            let _ = io::stdout().flush();

            // AI only suggests commands - it has NO access to contacts or messages
            match ai.chat(message) {
                Ok(result) => {
                    // Clear thinking indicator
                    print!("\r            \r");
                    let _ = io::stdout().flush();

                    // Show AI's explanation (if any)
                    if !result.display_text.is_empty() {
                        // Clean up the display text - remove any command echoes
                        let clean_text = result.display_text
                            .lines()
                            .filter(|line| !line.trim().starts_with('/'))
                            .collect::<Vec<_>>()
                            .join("\n")
                            .trim()
                            .to_string();

                        if !clean_text.is_empty() {
                            println!("{}", clean_text);
                        }
                    }

                    // Execute command captured directly from tool (reliable path)
                    if let Some(ref cmd) = result.command {
                        // Pass original message for feedback context
                        if let Ok(Some(feedback)) = self.execute_extracted_command(cmd, Some(message)) {
                            // If search returned no results, ask AI for alternative suggestion
                            if feedback.result_count == Some(0) {
                                self.handle_no_results_feedback(feedback)?;
                            }
                        }
                    } else {
                        // AI didn't call a tool - try to extract search intent from message
                        self.handle_no_tool_fallback(message)?;
                    }
                }
                Err(e) => {
                    // Clear thinking indicator
                    print!("\r            \r");
                    let _ = io::stdout().flush();

                    println!("AI error: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Handle case where AI responded but didn't call a tool
    /// Try to extract a simple search from the user's message
    fn handle_no_tool_fallback(&mut self, message: &str) -> Result<()> {
        // Look for location keywords
        let lower = message.to_lowercase();

        // Common patterns: "in <place>", "at <company>", "from <place>"
        let search_term = if let Some(pos) = lower.find(" in ") {
            let after = &message[pos + 4..];
            let term = after.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            if !term.is_empty() { Some(term) } else { None }
        } else if let Some(pos) = lower.find(" at ") {
            let after = &message[pos + 4..];
            let term = after.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            if !term.is_empty() { Some(format!("at {}", term)) } else { None }
        } else if let Some(pos) = lower.find(" from ") {
            let after = &message[pos + 6..];
            let term = after.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            if !term.is_empty() { Some(term) } else { None }
        } else {
            None
        };

        if let Some(term) = search_term {
            println!("(AI didn't search - trying: /search {})", term);
            let _ = self.do_search_with_prompt(&term);
        } else {
            println!("(Try: /search <term> or /list)");
        }

        Ok(())
    }

    /// Handle feedback when search returns no results
    /// Asks AI to suggest a simpler search and auto-executes it
    fn handle_no_results_feedback(&mut self, feedback: CommandFeedback) -> Result<()> {
        if let Some(ref mut ai) = self.ai_session {
            print!("\nSuggesting alternative...");
            let _ = io::stdout().flush();

            match ai.provide_feedback(feedback) {
                Ok(Some(suggestion)) => {
                    print!("\r                        \r");
                    let _ = io::stdout().flush();

                    // Extract command from text (fallback if not provided directly)
                    let mut explanation = String::new();
                    let mut text_cmd: Option<String> = None;

                    for line in suggestion.display_text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with('/') && text_cmd.is_none() {
                            text_cmd = Some(trimmed.to_string());
                        } else if !trimmed.is_empty() {
                            if !explanation.is_empty() {
                                explanation.push('\n');
                            }
                            explanation.push_str(trimmed);
                        }
                    }

                    // Show explanation if any
                    if !explanation.is_empty() {
                        println!("{}", explanation);
                    }

                    // Prefer command from tool call over text extraction
                    let cmd_to_run = suggestion.command.or(text_cmd);

                    // Auto-execute the suggested command (don't recurse on failure)
                    if let Some(cmd) = cmd_to_run {
                        println!("→ {}\n", cmd);
                        let parsed = ChatCommand::parse(&cmd);
                        if let ChatCommand::Search(query) = parsed {
                            if !query.is_empty() {
                                let _ = self.do_search_with_prompt(&query);
                            }
                        }
                    }
                }
                Ok(None) => {
                    print!("\r                        \r");
                    let _ = io::stdout().flush();
                }
                Err(_) => {
                    print!("\r                        \r");
                    let _ = io::stdout().flush();
                }
            }
        }
        Ok(())
    }

    /// Execute a command extracted from AI response
    /// Non-destructive commands run immediately; destructive ones ask for confirmation
    /// Returns feedback about the command execution for the AI
    fn execute_extracted_command(&mut self, cmd: &str, original_query: Option<&str>) -> Result<Option<CommandFeedback>> {
        println!("→ {}\n", cmd);

        let parsed = ChatCommand::parse(cmd);
        match parsed {
            // Non-destructive: execute immediately
            ChatCommand::Search(query) => {
                if !query.is_empty() {
                    let count = self.do_search_with_prompt(&query)?;
                    return Ok(Some(CommandFeedback::search_results(
                        cmd.to_string(),
                        count,
                        original_query.map(|s| s.to_string()),
                    )));
                }
            }
            ChatCommand::List => {
                self.do_list_with_prompt()?;
            }
            ChatCommand::Browse => {
                if self.last_results.is_empty() {
                    println!("No results to browse. Run a search first.");
                } else {
                    let count = self.last_results.len();
                    println!("Opening {} contact{}...\n", count, if count == 1 { "" } else { "s" });
                    run_browse(self.db, self.last_results.clone(), ViewMode::Card)?;
                    clear_screen()?;
                    self.print_welcome();
                }
            }
            ChatCommand::Messages(contact) => {
                if !contact.is_empty() {
                    // Search for the contact first, then show messages
                    let words: Vec<&str> = contact.split_whitespace().collect();
                    if let Ok(results) = self.db.search_persons_multi(&words, false, 1) {
                        if let Some(person) = results.first() {
                            if let Ok(Some(detail)) = self.db.get_contact_detail(person.id) {
                                println!("Opening messages for {}...\n",
                                    detail.person.display_name.as_deref().unwrap_or(&contact));
                                let _ = crate::cli::show::show_messages_screen(self.db, &detail);
                                clear_screen()?;
                                self.print_welcome();
                            }
                        } else {
                            println!("No contact found matching '{}'", contact);
                        }
                    }
                }
            }
            // Potentially destructive: would need confirmation (not implemented via AI)
            ChatCommand::Add => {
                println!("To add a contact, use: /add");
                println!("This will open the interactive add form.");
            }
            _ => {
                println!("Command: {}", cmd);
            }
        }
        Ok(None)
    }

    /// Search and show results - auto-execute, suggest appropriate next action
    /// Returns the number of results found (for feedback)
    fn do_search_with_prompt(&mut self, query: &str) -> Result<usize> {
        let words: Vec<&str> = query.split_whitespace().collect();
        match self.db.search_persons_multi(&words, false, u32::MAX) {
            Ok(results) => {
                if results.is_empty() {
                    println!("No matches for '{}'.", query);
                    self.last_results.clear();
                    Ok(0)
                } else {
                    let count = results.len();
                    self.last_results = results;

                    if count == 1 {
                        // Single result - show card directly (non-destructive, no confirmation needed)
                        println!("Found 1 match. Opening contact card...\n");
                        run_browse(self.db, self.last_results.clone(), ViewMode::Card)?;
                        clear_screen()?;
                        self.print_welcome();
                    } else if count <= 5 {
                        // Few results - show summary, suggest browse to take action
                        println!("Found {} contacts:\n", count);
                        self.print_results_summary(count);
                        println!("\n→ /browse to view and take action on these contacts");
                    } else {
                        // Many results - show preview, suggest browse
                        println!("Found {} contacts:\n", count);
                        self.print_results_summary(10);
                        println!("\n→ /browse to view all and take action");
                        println!("→ Or refine: /search <more specific terms>");
                    }
                    Ok(count)
                }
            }
            Err(e) => {
                println!("Search error: {}", e);
                self.last_results.clear();
                Ok(0)
            }
        }
    }

    /// List and show results
    fn do_list_with_prompt(&mut self) -> Result<()> {
        match self.db.list_persons(u32::MAX, 0) {
            Ok(persons) => {
                if persons.is_empty() {
                    println!("No contacts found.");
                } else {
                    let count = persons.len();
                    self.last_results = persons;

                    if count <= 10 {
                        println!("You have {} contacts:\n", count);
                        self.print_results_summary(count);
                    } else {
                        println!("You have {} contacts:\n", count);
                        self.print_results_summary(10);
                    }
                    println!("\n→ /browse to view and take action on contacts");
                }
            }
            Err(e) => println!("Error: {}", e),
        }
        Ok(())
    }

    /// Show a tutorial for the next feature to learn
    fn do_learn(&mut self) -> Result<()> {
        match self.db.get_next_to_learn() {
            Ok(Some(feature)) => {
                let is_refresher = feature.times_learned > 0;

                println!();
                if is_refresher {
                    println!("REFRESHER: {}", feature.tutorial.title);
                } else {
                    println!("LEARN: {}", feature.tutorial.title);
                }
                println!("─────────────────────────────────────");
                println!();
                println!("{}", feature.tutorial.summary);
                println!();

                println!("STEPS:");
                for (i, step) in feature.tutorial.steps.iter().enumerate() {
                    println!("  {}. {}", i + 1, step);
                }
                println!();

                if !feature.tutorial.tips.is_empty() {
                    println!("TIPS:");
                    for tip in &feature.tutorial.tips {
                        println!("  - {}", tip);
                    }
                    println!();
                }

                if !feature.tutorial.related_features.is_empty() {
                    println!("Related: {}", feature.tutorial.related_features.join(", "));
                    println!();
                }

                // Mark as learned
                self.db.mark_feature_learned(&feature.id)?;

                // Show progress
                if let Ok((learned, total)) = self.db.get_learning_stats() {
                    println!("Progress: {}/{} features learned", learned, total);
                }
                println!();
            }
            Ok(None) => {
                println!("No tutorials available.");
                println!();
            }
            Err(e) => {
                println!("Error loading tutorial: {}", e);
                println!();
            }
        }
        Ok(())
    }

    /// Show a tutorial for a specific topic
    fn do_teach(&self, topic: &str) -> Result<()> {
        match self.db.find_feature_by_name(topic) {
            Ok(Some(feature)) => {
                println!();
                println!("TUTORIAL: {}", feature.tutorial.title);
                println!("─────────────────────────────────────");
                println!();
                println!("{}", feature.tutorial.summary);
                println!();

                println!("STEPS:");
                for (i, step) in feature.tutorial.steps.iter().enumerate() {
                    println!("  {}. {}", i + 1, step);
                }
                println!();

                if !feature.tutorial.tips.is_empty() {
                    println!("TIPS:");
                    for tip in &feature.tutorial.tips {
                        println!("  - {}", tip);
                    }
                    println!();
                }

                if !feature.tutorial.related_features.is_empty() {
                    println!("Related: {}", feature.tutorial.related_features.join(", "));
                }
            }
            Ok(None) => {
                println!("No tutorial found for '{}'. Try:", topic);
                println!("  /teach search");
                println!("  /teach messages");
                println!("  /teach sync");
                println!("  /teach import");
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        Ok(())
    }

    /// Show recent SMS/iMessage contacts
    fn do_recent(&mut self, days: u32) -> Result<()> {
        use crate::cli::messages::{get_recent_message_handles, DetectedService};
        use std::collections::HashMap;
        use chrono::Local;

        println!();

        // Get recent handles from iMessage database
        let handles = match get_recent_message_handles(days) {
            Ok(h) => h,
            Err(e) => {
                println!("Error reading Messages database: {}", e);
                println!("(Make sure you've granted Full Disk Access to Terminal)");
                println!();
                return Ok(());
            }
        };

        if handles.is_empty() {
            println!("No messages found in the last {} day{}.", days, if days == 1 { "" } else { "s" });
            println!();
            return Ok(());
        }

        println!("Recent contacts (last {} day{}):\n", days, if days == 1 { "" } else { "s" });

        // Build lookup maps once (O(persons) queries instead of O(handles × persons))
        let all_persons = self.db.list_persons(u32::MAX, 0)?;
        let mut phone_to_person: HashMap<String, Person> = HashMap::new();
        let mut email_to_person: HashMap<String, Person> = HashMap::new();

        for person in &all_persons {
            if let Ok(phones) = self.db.get_phones_for_person(person.id) {
                for phone in phones {
                    // Normalize phone for lookup
                    let normalized: String = phone.phone_number.chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    if normalized.len() >= 10 {
                        // Store last 10 digits as key
                        let key = if normalized.len() > 10 {
                            normalized[normalized.len()-10..].to_string()
                        } else {
                            normalized
                        };
                        phone_to_person.entry(key).or_insert_with(|| person.clone());
                    }
                }
            }
            if let Ok(emails) = self.db.get_emails_for_person(person.id) {
                for email in emails {
                    email_to_person.entry(email.email_address.to_lowercase())
                        .or_insert_with(|| person.clone());
                }
            }
        }

        let mut matched_persons: Vec<Person> = Vec::new();
        let mut unknown_count = 0;
        let now = Local::now();

        for rh in &handles {
            // Try to match handle to a person via lookup maps
            let found_person = {
                // Try phone lookup first
                let handle_digits: String = rh.handle.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                let phone_key = if handle_digits.len() > 10 {
                    handle_digits[handle_digits.len()-10..].to_string()
                } else {
                    handle_digits.clone()
                };

                phone_to_person.get(&phone_key).cloned()
                    .or_else(|| email_to_person.get(&rh.handle.to_lowercase()).cloned())
            };

            // Format relative time
            let duration = now.signed_duration_since(rh.last_message_date);
            let time_ago = if duration.num_days() == 0 {
                "today".to_string()
            } else if duration.num_days() == 1 {
                "1 day ago".to_string()
            } else {
                format!("{} days ago", duration.num_days())
            };

            // Format service
            let service_str = match rh.service {
                DetectedService::IMessage => "iMessage",
                DetectedService::Sms => "SMS",
                DetectedService::Unknown => "Message",
            };

            if let Some(person) = found_person {
                let name = person.display_name.as_deref().unwrap_or(&rh.handle);
                println!("  {:20} {:12}  {}", name, time_ago, service_str);
                if !matched_persons.iter().any(|p| p.id == person.id) {
                    matched_persons.push(person);
                }
            } else {
                println!("  {:20} {:12}  {}  (unknown)", rh.handle, time_ago, service_str);
                unknown_count += 1;
            }
        }

        // Store matched persons for /browse
        self.last_results = matched_persons;
        let total = handles.len();
        let known = total - unknown_count;

        println!();
        if known > 0 {
            println!("{} contact{}{}. /browse to view details.",
                known,
                if known == 1 { "" } else { "s" },
                if unknown_count > 0 { format!(", {} unknown", unknown_count) } else { String::new() }
            );
        } else {
            println!("{} message{}, none matched to contacts.", total, if total == 1 { "" } else { "s" });
        }
        println!();

        Ok(())
    }

    /// Print a summary of stored results
    fn print_results_summary(&self, limit: usize) {
        for (i, person) in self.last_results.iter().take(limit).enumerate() {
            let name = person.display_name.as_deref().unwrap_or("(unnamed)");

            // Try to get more detail for display
            if let Ok(Some(detail)) = self.db.get_contact_detail(person.id) {
                let email = detail.emails.first().map(|e| e.email_address.as_str());
                let org_title = if let Some((person_org, org)) = detail.organizations.first() {
                    let title = person_org.title.as_deref().unwrap_or("");
                    let org_name = &org.name;
                    if !title.is_empty() && !org_name.is_empty() {
                        Some(format!("{} at {}", title, org_name))
                    } else if !org_name.is_empty() {
                        Some(org_name.clone())
                    } else if !title.is_empty() {
                        Some(title.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                print!("  {}. {}", i + 1, name);
                if let Some(e) = email {
                    print!(" ({})", e);
                }
                if let Some(ot) = org_title {
                    print!(" - {}", ot);
                }
                println!();
            } else {
                println!("  {}. {}", i + 1, name);
            }
        }

        if self.last_results.len() > limit {
            println!("  ... and {} more", self.last_results.len() - limit);
        }
    }
}

/// Action to take after executing a command
#[allow(dead_code)]
enum ChatAction {
    Continue,
    Quit,
    Back,
}

/// Run the chat interface
pub fn run_chat(db: &Database) -> Result<bool> {
    let mut session = ChatSession::new(db);
    session.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(ChatCommand::parse("/help"), ChatCommand::Help);
        assert_eq!(ChatCommand::parse("/h"), ChatCommand::Help);
        assert_eq!(ChatCommand::parse("/?"), ChatCommand::Help);
        assert_eq!(ChatCommand::parse("/HELP"), ChatCommand::Help);
    }

    #[test]
    fn test_parse_search() {
        assert_eq!(ChatCommand::parse("/search john"), ChatCommand::Search("john".to_string()));
        assert_eq!(ChatCommand::parse("/s jane doe"), ChatCommand::Search("jane doe".to_string()));
        assert_eq!(ChatCommand::parse("/find bob"), ChatCommand::Search("bob".to_string()));
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(ChatCommand::parse("/list"), ChatCommand::List);
        assert_eq!(ChatCommand::parse("/l"), ChatCommand::List);
        assert_eq!(ChatCommand::parse("/ls"), ChatCommand::List);
    }

    #[test]
    fn test_parse_browse() {
        assert_eq!(ChatCommand::parse("/browse"), ChatCommand::Browse);
        assert_eq!(ChatCommand::parse("/b"), ChatCommand::Browse);
    }

    #[test]
    fn test_parse_recent() {
        assert_eq!(ChatCommand::parse("/recent"), ChatCommand::Recent(7));
        assert_eq!(ChatCommand::parse("/r"), ChatCommand::Recent(7));
        assert_eq!(ChatCommand::parse("/recent 14"), ChatCommand::Recent(14));
        assert_eq!(ChatCommand::parse("/r 30"), ChatCommand::Recent(30));
        // Invalid number falls back to default
        assert_eq!(ChatCommand::parse("/recent abc"), ChatCommand::Recent(7));
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(ChatCommand::parse("/quit"), ChatCommand::Quit);
        assert_eq!(ChatCommand::parse("/q"), ChatCommand::Quit);
        assert_eq!(ChatCommand::parse("/exit"), ChatCommand::Quit);
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(ChatCommand::parse(""), ChatCommand::Empty);
        assert_eq!(ChatCommand::parse("   "), ChatCommand::Empty);
    }

    #[test]
    fn test_parse_chat_not_command() {
        // Natural language should go to AI, not be parsed as commands
        assert_eq!(ChatCommand::parse("I need to find Greg"), ChatCommand::Unknown("I need to find Greg".to_string()));
        assert_eq!(ChatCommand::parse("help me find someone"), ChatCommand::Unknown("help me find someone".to_string()));
        assert_eq!(ChatCommand::parse("import my contacts please"), ChatCommand::Unknown("import my contacts please".to_string()));
    }

    #[test]
    fn test_parse_unknown_command() {
        assert_eq!(ChatCommand::parse("/foobar"), ChatCommand::Unknown("/foobar".to_string()));
    }
}
