//! Main menu for contactcmd
//!
//! Uses inquire for clean, reliable terminal interaction.

use anyhow::{anyhow, Result};
use inquire::{Select, Text};
use std::io::{self, IsTerminal};

use crate::cli::ui::{clear_screen, minimal_render_config, search_input_combined};
use crate::cli::chat::run_chat;
use crate::cli::gateway::approve::run_approve;
use crate::cli::{pick_csv_file, run_add, run_cleanup, run_import, run_messages, run_search, run_setup, run_show, run_sync, run_tasks};
use crate::cli::list::{run_browse, ViewMode};
use crate::db::Database;

/// Menu options with type-safe variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuOption {
    Chat,
    Tasks,
    Gateway,
    Browse,
    BrowseByTag,
    Search,
    Show,
    Add,
    Import,
    MissingEmail,
    MissingPhone,
    Cleanup,
    Sync,
    Messages,
    Setup,
    Quit,
}

impl MenuOption {
    const ALL: &'static [MenuOption] = &[
        MenuOption::Chat,
        MenuOption::Tasks,
        MenuOption::Gateway,
        MenuOption::Browse,
        MenuOption::BrowseByTag,
        MenuOption::Search,
        MenuOption::Show,
        MenuOption::Add,
        MenuOption::Import,
        MenuOption::MissingEmail,
        MenuOption::MissingPhone,
        MenuOption::Cleanup,
        MenuOption::Sync,
        MenuOption::Messages,
        MenuOption::Setup,
        MenuOption::Quit,
    ];

    fn label(self) -> &'static str {
        match self {
            MenuOption::Chat => "Chat",
            MenuOption::Tasks => "Tasks",
            MenuOption::Gateway => "Gateway",
            MenuOption::Browse => "Browse",
            MenuOption::BrowseByTag => "Browse by Tag",
            MenuOption::Search => "Search",
            MenuOption::Show => "Show",
            MenuOption::Add => "Add",
            MenuOption::Import => "Import",
            MenuOption::MissingEmail => "Missing Email",
            MenuOption::MissingPhone => "Missing Phone",
            MenuOption::Cleanup => "Cleanup",
            MenuOption::Sync => "Sync",
            MenuOption::Messages => "Messages",
            MenuOption::Setup => "Setup",
            MenuOption::Quit => "Quit",
        }
    }

    fn from_label(s: &str) -> Option<MenuOption> {
        // Handle labels with counts like "Gateway (3)"
        let base_label = s.split(" (").next().unwrap_or(s);
        MenuOption::ALL.iter().find(|opt| opt.label() == base_label).copied()
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

    loop {
        // Clear screen - if this fails, continue anyway (degraded but functional)
        let _ = clear_screen();

        // Build menu labels with dynamic counts
        let gateway_pending = db.count_pending_queue().unwrap_or(0);
        let menu_labels: Vec<String> = MenuOption::ALL.iter().map(|opt| {
            if *opt == MenuOption::Gateway && gateway_pending > 0 {
                format!("{} ({})", opt.label(), gateway_pending)
            } else {
                opt.label().to_string()
            }
        }).collect();
        let menu_refs: Vec<&str> = menu_labels.iter().map(|s| s.as_str()).collect();
        let menu_len = menu_refs.len();

        let selection = Select::new("contactcmd", menu_refs)
            .with_render_config(minimal_render_config())
            .with_page_size(menu_len)
            .with_vim_mode(true)
            .without_filtering()
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
        MenuOption::Chat => {
            run_chat(db)
        }
        MenuOption::Tasks => {
            run_tasks(db)
        }
        MenuOption::Gateway => {
            let pending = db.count_pending_queue()?;
            if pending == 0 {
                println!("No pending messages in gateway queue.");
                wait_for_continue();
                Ok(false)
            } else {
                run_approve(db)
            }
        }
        MenuOption::Browse => {
            let persons = db.list_persons(10000, 0)?;
            run_browse(db, persons, ViewMode::Card).map(|_| false)
        }
        MenuOption::BrowseByTag => {
            let tags = db.list_tags()?;
            if tags.is_empty() {
                println!("No tags defined yet.");
                wait_for_continue();
                return Ok(false);
            }

            let options: Vec<String> = tags.iter().map(|t| {
                let count = db.get_persons_by_tag(&t.name).map(|p| p.len()).unwrap_or(0);
                format!("{} ({} contacts)", t.name, count)
            }).collect();

            let selection = Select::new("Select a tag:", options)
                .with_render_config(minimal_render_config())
                .without_filtering()
                .prompt_skippable()?;

            let Some(selected) = selection else {
                return Ok(false);
            };

            // Extract tag name (before the " (")
            let tag_name = selected.split(" (").next().unwrap_or(&selected);
            let persons = db.get_persons_by_tag(tag_name)?;

            if persons.is_empty() {
                println!("No contacts with tag '{}'", tag_name);
                wait_for_continue();
                return Ok(false);
            }

            run_browse(db, persons, ViewMode::Card).map(|_| false)
        }
        MenuOption::Search => {
            // Combined search input: text field + filter selector on same screen
            let result = search_input_combined("search: ")?;

            let Some(input) = result else {
                return Ok(false);
            };

            let query = input.query.trim();
            if query.is_empty() {
                let persons = db.list_persons(10000, 0)?;
                return run_browse(db, persons, ViewMode::Table).map(|_| false);
            }

            // Handle Tag filter specially - prompt for tag selection
            if input.field.is_tag() {
                let tags = db.list_tags()?;
                if tags.is_empty() {
                    println!("No tags defined yet.");
                    wait_for_continue();
                    return Ok(false);
                }

                let options: Vec<String> = tags.iter().map(|t| {
                    let count = db.get_persons_by_tag(&t.name).map(|p| p.len()).unwrap_or(0);
                    format!("{} ({} contacts)", t.name, count)
                }).collect();

                let selection = Select::new("Select tag:", options)
                    .with_render_config(minimal_render_config())
                    .without_filtering()
                    .prompt_skippable()?;

                let Some(selected) = selection else {
                    return Ok(false);
                };

                let tag_name = selected.split(" (").next().unwrap_or(&selected);

                // Search within tagged contacts
                let persons = db.get_persons_by_tag(tag_name)?;
                let query_lower = query.to_lowercase();
                let filtered: Vec<_> = persons
                    .into_iter()
                    .filter(|p| {
                        p.search_name.as_ref().map(|n| n.contains(&query_lower)).unwrap_or(false)
                            || p.display_name.as_ref().map(|n| n.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    })
                    .collect();

                if filtered.is_empty() {
                    println!("No matches for '{}' in tag '{}'", query, tag_name);
                    wait_for_continue();
                    return Ok(false);
                }

                return run_browse(db, filtered, ViewMode::Card).map(|_| false);
            }

            // Regular field search
            run_search(db, query, false, None, input.field.to_field_str()).map(|_| false)
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
        MenuOption::Import => {
            let options = vec!["Browse for file", "Enter path manually"];
            let choice = Select::new("How would you like to select the CSV file?", options)
                .with_render_config(minimal_render_config())
                .without_filtering()
                .prompt_skippable()?;

            let file = match choice.as_deref() {
                Some("Browse for file") => {
                    match pick_csv_file() {
                        Some(path) => path,
                        None => {
                            println!("No file selected.");
                            return Ok(false);
                        }
                    }
                }
                Some("Enter path manually") => {
                    let path = prompt_for_input("CSV file path: ")?;
                    if path.is_empty() {
                        return Ok(false);
                    }
                    path
                }
                _ => return Ok(false),
            };

            let dry_run = inquire::Confirm::new("Dry run?")
                .with_default(true)
                .with_render_config(minimal_render_config())
                .prompt_skippable()?
                .unwrap_or(true);

            run_import(db, &file, dry_run, None)?;
            wait_for_continue();
            Ok(false)
        }
        MenuOption::MissingEmail => {
            run_search(db, "", false, Some("email"), None).map(|_| false)
        }
        MenuOption::MissingPhone => {
            run_search(db, "", false, Some("phone"), None).map(|_| false)
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
        MenuOption::Setup => {
            run_setup(db).map(|_| false)
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
        assert_eq!(MenuOption::ALL.len(), 16);
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
