//! Main menu for contactcmd
//!
//! Uses inquire for clean, reliable terminal interaction.

use anyhow::{anyhow, Result};
use inquire::{Select, Text};
use std::io::{self, IsTerminal};

use crate::cli::ui::{clear_screen, minimal_render_config};
use crate::cli::{run_add, run_browse_mode, run_cleanup, run_list, run_messages, run_search, run_show, run_sync};
use crate::db::Database;

/// Menu options with type-safe variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuOption {
    Browse,
    List,
    Search,
    Show,
    Add,
    MissingEmail,
    MissingPhone,
    Cleanup,
    Sync,
    Messages,
    Quit,
}

impl MenuOption {
    const ALL: &'static [MenuOption] = &[
        MenuOption::Browse,
        MenuOption::List,
        MenuOption::Search,
        MenuOption::Show,
        MenuOption::Add,
        MenuOption::MissingEmail,
        MenuOption::MissingPhone,
        MenuOption::Cleanup,
        MenuOption::Sync,
        MenuOption::Messages,
        MenuOption::Quit,
    ];

    fn label(self) -> &'static str {
        match self {
            MenuOption::Browse => "Browse",
            MenuOption::List => "List",
            MenuOption::Search => "Search",
            MenuOption::Show => "Show",
            MenuOption::Add => "Add",
            MenuOption::MissingEmail => "Missing Email",
            MenuOption::MissingPhone => "Missing Phone",
            MenuOption::Cleanup => "Cleanup",
            MenuOption::Sync => "Sync",
            MenuOption::Messages => "Messages",
            MenuOption::Quit => "Quit",
        }
    }

    fn from_label(s: &str) -> Option<MenuOption> {
        MenuOption::ALL.iter().find(|opt| opt.label() == s).copied()
    }
}

/// Run the interactive main menu
pub fn run_menu(db: &Database) -> Result<()> {
    // TTY check: interactive menu requires a terminal
    if !io::stdin().is_terminal() {
        return Err(anyhow!(
            "Interactive menu requires a terminal. Use subcommands for non-interactive use:\n  \
            contactcmd list\n  \
            contactcmd search <query>\n  \
            contactcmd show <name>\n  \
            Run 'contactcmd --help' for all options."
        ));
    }

    let menu_labels: Vec<&str> = MenuOption::ALL.iter().map(|opt| opt.label()).collect();

    loop {
        // Clear screen - if this fails, continue anyway (degraded but functional)
        let _ = clear_screen();

        let selection = Select::new("contactcmd", menu_labels.clone())
            .with_render_config(minimal_render_config())
            .with_page_size(menu_labels.len())
            .with_vim_mode(true)
            .prompt_skippable();

        // Handle prompt errors (Ctrl+C, terminal issues) - exit gracefully
        let selection = match selection {
            Ok(sel) => sel,
            Err(_) => return Ok(()),
        };

        let Some(choice_label) = selection else {
            // User pressed Escape
            return Ok(());
        };

        let Some(choice) = MenuOption::from_label(choice_label) else {
            // Should never happen with type-safe menu, but handle gracefully
            continue;
        };

        if choice == MenuOption::Quit {
            return Ok(());
        }

        let _ = clear_screen();

        // Execute command - all errors caught and displayed
        // Returns true if user wants to quit the app
        let result = execute_command(db, choice);

        match result {
            Ok(true) => return Ok(()), // User pressed quit
            Err(e) => {
                eprintln!("\nError: {}", e);
                wait_for_continue();
            }
            _ => {}
        }
    }
}

/// Execute a menu command, catching all errors
/// Returns Ok(true) if the user wants to quit the app
fn execute_command(db: &Database, choice: MenuOption) -> Result<bool> {
    match choice {
        MenuOption::Browse => {
            let persons = db.list_persons(10000, 0)?;
            run_browse_mode(db, persons).map(|_| false)
        }
        MenuOption::List => {
            run_list(db, 1, 0, None, "asc".into(), false, false).map(|_| false)
        }
        MenuOption::Search => {
            let query = prompt_for_input("search: ")?;
            if query.is_empty() {
                // Empty search shows all contacts (becomes list)
                run_list(db, 1, 20, None, "asc".into(), false, true).map(|_| false)
            } else {
                run_search(db, &query, false, None).map(|_| false)
            }
        }
        MenuOption::Show => {
            let name = prompt_for_input("name: ")?;
            if name.is_empty() {
                return Ok(false);
            }
            run_show(db, &name)
        }
        MenuOption::Add => {
            run_add(db, None, None, None, None, None).map(|_| false)
        }
        MenuOption::MissingEmail => {
            run_search(db, "", false, Some("email")).map(|_| false)
        }
        MenuOption::MissingPhone => {
            run_search(db, "", false, Some("phone")).map(|_| false)
        }
        MenuOption::Cleanup => {
            run_cleanup(db).map(|_| false)
        }
        MenuOption::Sync => {
            match detect_sync_source() {
                Some(source) => run_sync(db, source, false).map(|_| false),
                None => {
                    println!("Sync is only available on macOS.");
                    Ok(false)
                }
            }
        }
        MenuOption::Messages => {
            let query = prompt_for_input("search: ")?;
            if query.is_empty() {
                return Ok(false);
            }
            run_messages(db, &query, None).map(|_| false)
        }
        MenuOption::Quit => Ok(true),
    }
}

/// Prompt for text input, returning empty string on cancel
fn prompt_for_input(label: &str) -> Result<String> {
    let result = Text::new(label)
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;
    Ok(result.unwrap_or_default())
}

/// Wait for user to press enter to continue
fn wait_for_continue() {
    println!();
    let _ = Text::new("[enter]")
        .with_render_config(minimal_render_config())
        .prompt_skippable();
}

/// Detect the appropriate sync source for the current platform
fn detect_sync_source() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    {
        Some("mac")
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_option_roundtrip() {
        for opt in MenuOption::ALL {
            let label = opt.label();
            let recovered = MenuOption::from_label(label);
            assert_eq!(recovered, Some(*opt), "Failed roundtrip for {:?}", opt);
        }
    }

    #[test]
    fn test_menu_option_from_invalid_label() {
        assert_eq!(MenuOption::from_label("Invalid"), None);
        assert_eq!(MenuOption::from_label(""), None);
    }

    #[test]
    fn test_menu_option_all_has_correct_count() {
        assert_eq!(MenuOption::ALL.len(), 11);
    }

    #[test]
    fn test_detect_sync_source() {
        let source = detect_sync_source();
        #[cfg(target_os = "macos")]
        assert_eq!(source, Some("mac"));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(source, None);
    }
}
