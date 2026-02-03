//! Task management UI for contactcmd
//!
//! Implements an interactive task list with Eisenhower matrix quadrants.
//! Integrates into the main menu as "Tasks".

use anyhow::Result;
use chrono::{DateTime, Datelike, Local, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Attribute, SetAttribute},
    ExecutableCommand,
};
use inquire::{Select, Text};
use std::io::{self, Write};

use crate::cli::ui::{clear_screen, minimal_render_config, confirm, truncate, RawModeGuard, StatusBar};
use crate::db::Database;
use crate::models::Task;
use uuid::Uuid;

/// Sort mode for task list
#[derive(Debug, Clone, Copy, PartialEq)]
enum SortMode {
    Quadrant,
    Deadline,
}

/// Entry point from menu - returns true if user wants to quit app
pub fn run_tasks(db: &Database) -> Result<bool> {
    run_task_list(db)
}

/// Interactive task list with keyboard navigation
pub fn run_task_list(db: &Database) -> Result<bool> {
    let mut sort_mode = SortMode::Quadrant;
    let mut show_completed = false;
    let mut selected_idx: usize = 0;

    loop {
        // Load tasks based on sort mode
        let tasks = match sort_mode {
            SortMode::Quadrant => db.list_tasks(show_completed)?,
            SortMode::Deadline => db.list_tasks_by_deadline(show_completed)?,
        };

        // Get counts for status bar
        let pending_count = db.count_pending_tasks()?;
        let completed_count = db.count_completed_tasks()?;

        // Clamp selection to valid range
        if tasks.is_empty() {
            selected_idx = 0;
        } else if selected_idx >= tasks.len() {
            selected_idx = tasks.len().saturating_sub(1);
        }

        // Render the task list
        clear_screen()?;
        let mut stdout = io::stdout();

        // Header
        let header = format!(
            "TASKS ({} pending{})\n",
            pending_count,
            if show_completed { format!(", {} done", completed_count) } else { String::new() }
        );
        print!("{}", header);

        // Table header
        print_task_header();

        if tasks.is_empty() {
            println!("  No tasks. Press [n] to add one.\n");
        } else {
            // Render based on sort mode
            match sort_mode {
                SortMode::Quadrant => {
                    render_by_quadrant(&mut stdout, &tasks, selected_idx)?;
                }
                SortMode::Deadline => {
                    render_by_deadline(&mut stdout, &tasks, selected_idx)?;
                }
            }
        }

        // Status bar
        println!();
        let sort_hint = match sort_mode {
            SortMode::Quadrant => "by:date",
            SortMode::Deadline => "by:quad",
        };
        let status = StatusBar::new()
            .counter(if tasks.is_empty() { 0 } else { selected_idx + 1 }, tasks.len())
            .action("n", "ew")
            .action("c", "heck")
            .action("d", "el")
            .action("h", "ide done")
            .action("s", sort_hint)
            .action("↑/↓", "")
            .action("q", "/esc")
            .action("Q", "uit")
            .render();
        println!("{}", status);
        stdout.flush()?;

        // Handle input
        let code = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    // Handle Ctrl+C in raw mode
                    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(false);
                    }
                    code
                }
                _ => continue,
            }
        };

        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(false); // Return to menu
            }
            KeyCode::Char('Q') => {
                return Ok(true); // Quit app
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected_idx = selected_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !tasks.is_empty() && selected_idx < tasks.len() - 1 {
                    selected_idx += 1;
                }
            }
            KeyCode::Char('n') => {
                // Add new task
                task_add_prompt(db)?;
            }
            KeyCode::Char('c') => {
                // Toggle complete
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    if task.is_completed() {
                        db.uncomplete_task(task.id)?;
                    } else {
                        db.complete_task(task.id)?;
                    }
                }
            }
            KeyCode::Char('d') => {
                // Delete with confirm
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    let _ = clear_screen();
                    if confirm(&format!("Delete \"{}\"?", task.title))? {
                        db.delete_task(task.id)?;
                    }
                }
            }
            KeyCode::Char('s') => {
                // Toggle sort mode
                sort_mode = match sort_mode {
                    SortMode::Quadrant => SortMode::Deadline,
                    SortMode::Deadline => SortMode::Quadrant,
                };
            }
            KeyCode::Char('h') => {
                // Toggle show completed
                show_completed = !show_completed;
            }
            KeyCode::Enter => {
                // View/edit task detail
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    if let Some(updated) = task_detail_screen(db, task)? {
                        db.update_task(&updated)?;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Print table header for tasks
fn print_task_header() {
    let layout = TaskLayout::default();
    println!(
        "{:<check$}  {:<title$}  {:<date$}  Q",
        "",
        "TASK",
        "DUE/DONE",
        check = layout.check_width,
        title = layout.title_width,
        date = layout.date_width
    );
}

/// Column layout for task display
struct TaskLayout {
    check_width: usize,
    title_width: usize,
    date_width: usize,
}

impl Default for TaskLayout {
    fn default() -> Self {
        let term_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);

        if term_width >= 80 {
            TaskLayout {
                check_width: 3,
                title_width: 40,
                date_width: 16,
            }
        } else {
            TaskLayout {
                check_width: 3,
                title_width: 30,
                date_width: 12,
            }
        }
    }
}

/// Render tasks grouped by quadrant
fn render_by_quadrant(stdout: &mut io::Stdout, tasks: &[Task], selected_idx: usize) -> Result<()> {
    let mut current_quadrant: Option<u8> = None;

    for (idx, task) in tasks.iter().enumerate() {
        // Print quadrant header if changed
        if current_quadrant != Some(task.quadrant) {
            if current_quadrant.is_some() {
                println!(); // Blank line between quadrants
            }
            println!("--- {} ---", quadrant_header(task.quadrant));
            current_quadrant = Some(task.quadrant);
        }

        // Print task row with reverse video if selected
        print_task_row(stdout, task, idx == selected_idx)?;
    }
    Ok(())
}

/// Render tasks sorted by deadline
fn render_by_deadline(stdout: &mut io::Stdout, tasks: &[Task], selected_idx: usize) -> Result<()> {
    for (idx, task) in tasks.iter().enumerate() {
        print_task_row(stdout, task, idx == selected_idx)?;
    }
    Ok(())
}

/// Print a single task row, with reverse video if selected
fn print_task_row(stdout: &mut io::Stdout, task: &Task, selected: bool) -> Result<()> {
    let layout = TaskLayout::default();

    // Checkbox: [x] for done, [ ] for pending
    let checkbox = if task.is_completed() { "[x]" } else { "[ ]" };

    // Title (truncated)
    let title = truncate(&task.title, layout.title_width);

    // Date column
    let date_str = if let Some(completed) = task.completed_at {
        let local = completed.with_timezone(&Local);
        format!("done {}", format_short_date(local))
    } else if let Some(deadline) = task.deadline {
        format_deadline(deadline)
    } else {
        String::new()
    };
    let date_display = truncate(&date_str, layout.date_width);

    // Quadrant
    let q = task.quadrant;

    let line = format!(
        "{:<check$}  {:<title$}  {:<date$}  {}",
        checkbox,
        title,
        date_display,
        q,
        check = layout.check_width,
        title = layout.title_width,
        date = layout.date_width
    );

    if selected {
        stdout.execute(SetAttribute(Attribute::Reverse))?;
        print!("{}", line);
        stdout.execute(SetAttribute(Attribute::Reset))?;
        println!();
    } else {
        println!("{}", line);
    }

    Ok(())
}

/// Format deadline for display
fn format_deadline(deadline: DateTime<Utc>) -> String {
    let local = deadline.with_timezone(&Local);
    let now = Local::now();
    let today = now.date_naive();
    let deadline_date = local.date_naive();

    if deadline_date == today {
        "today".to_string()
    } else if deadline_date == today.succ_opt().unwrap_or(today) {
        "tomorrow".to_string()
    } else if deadline_date < today {
        format!("{} (overdue)", format_short_date(local))
    } else {
        format_short_date(local)
    }
}

/// Format a date for short display
fn format_short_date(dt: DateTime<Local>) -> String {
    let now = Local::now();
    if dt.year() == now.year() {
        format!("{} {}", month_abbrev(dt.month()), dt.day())
    } else {
        format!("{} {}, {}", month_abbrev(dt.month()), dt.day(), dt.year())
    }
}

fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "???",
    }
}

/// Get quadrant header text
fn quadrant_header(quadrant: u8) -> &'static str {
    match quadrant {
        1 => "Q1: Urgent & Important",
        2 => "Q2: Important",
        3 => "Q3: Urgent",
        _ => "Q4: Later",
    }
}

/// Prompt to add a new task
pub fn task_add_prompt(db: &Database) -> Result<()> {
    let _ = clear_screen();

    // Title (required)
    let title = Text::new("title:")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let Some(title) = title else {
        return Ok(()); // Cancelled
    };

    let title = title.trim();
    if title.is_empty() {
        return Ok(()); // Empty title
    }

    // Quadrant selection
    let quadrant_options = vec![
        "1. Urgent & Important",
        "2. Important",
        "3. Urgent",
        "4. Later (default)",
    ];

    let quadrant_result = Select::new("quadrant:", quadrant_options)
        .with_render_config(minimal_render_config())
        .with_starting_cursor(3) // Default to Q4
        .prompt_skippable()?;

    let quadrant = match quadrant_result {
        Some(s) if s.starts_with("1.") => 1,
        Some(s) if s.starts_with("2.") => 2,
        Some(s) if s.starts_with("3.") => 3,
        _ => 4,
    };

    // Deadline (optional)
    let deadline_str = Text::new("deadline (YYYY-MM-DD, optional):")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let deadline = deadline_str
        .and_then(|s| parse_date_input(&s));

    // Create and save task
    let mut task = Task::new(title.to_string());
    task.quadrant = quadrant;
    task.deadline = deadline;

    db.insert_task(&task)?;
    Ok(())
}

/// Parse date input in various formats
fn parse_date_input(input: &str) -> Option<DateTime<Utc>> {
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return None;
    }

    // Handle special keywords
    let today = Local::now().date_naive();
    let date = match input.as_str() {
        "today" => today,
        "tomorrow" => today.succ_opt()?,
        _ => {
            // Try YYYY-MM-DD format
            chrono::NaiveDate::parse_from_str(&input, "%Y-%m-%d").ok()?
        }
    };

    // Convert to end of day UTC
    let datetime = date.and_hms_opt(23, 59, 59)?;
    Some(DateTime::from_naive_utc_and_offset(datetime, Utc))
}

/// Interactive task list filtered to a specific contact
/// Returns Ok(false) to return to contact card, Ok(true) to quit app
pub fn run_tasks_for_contact(db: &Database, person_id: Uuid, person_name: &str) -> Result<bool> {
    let mut sort_mode = SortMode::Quadrant;
    let mut selected_idx: usize = 0;

    loop {
        // Load tasks for this person
        let all_tasks = db.get_tasks_for_person(person_id)?;
        let tasks: Vec<Task> = match sort_mode {
            SortMode::Quadrant => all_tasks,
            SortMode::Deadline => {
                let mut sorted = all_tasks;
                sorted.sort_by(|a, b| {
                    match (&a.deadline, &b.deadline) {
                        (Some(da), Some(db)) => da.cmp(db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.quadrant.cmp(&b.quadrant),
                    }
                });
                sorted
            }
        };

        // Count pending tasks
        let pending_count = tasks.iter().filter(|t| !t.is_completed()).count();

        // Clamp selection
        if tasks.is_empty() {
            selected_idx = 0;
        } else if selected_idx >= tasks.len() {
            selected_idx = tasks.len().saturating_sub(1);
        }

        // Render
        clear_screen()?;
        let mut stdout = io::stdout();

        // Header shows contact name
        let header = format!("TASKS for {} ({} pending)\n", person_name, pending_count);
        print!("{}", header);

        print_task_header();

        if tasks.is_empty() {
            println!("  No tasks for this contact. Press [a] to add one.\n");
        } else {
            match sort_mode {
                SortMode::Quadrant => {
                    render_by_quadrant(&mut stdout, &tasks, selected_idx)?;
                }
                SortMode::Deadline => {
                    render_by_deadline(&mut stdout, &tasks, selected_idx)?;
                }
            }
        }

        // Status bar
        println!();
        let sort_hint = match sort_mode {
            SortMode::Quadrant => "by:date",
            SortMode::Deadline => "by:quad",
        };
        let status = StatusBar::new()
            .counter(if tasks.is_empty() { 0 } else { selected_idx + 1 }, tasks.len())
            .action("a", "dd")
            .action("l", "ink")
            .action("c", "heck")
            .action("u", "nlink")
            .action("s", sort_hint)
            .action("↑/↓", "")
            .action("q", "/esc")
            .action("Q", "uit")
            .render();
        println!("{}", status);
        stdout.flush()?;

        // Handle input
        let code = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(false);
                    }
                    code
                }
                _ => continue,
            }
        };

        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(false); // Return to contact card
            }
            KeyCode::Char('Q') => {
                return Ok(true); // Quit app
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected_idx = selected_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !tasks.is_empty() && selected_idx < tasks.len() - 1 {
                    selected_idx += 1;
                }
            }
            KeyCode::Char('a') => {
                // Quick add: just title, defaults to Q4
                task_quick_add_for_contact(db, person_id)?;
            }
            KeyCode::Char('l') => {
                // Link an existing unlinked task to this contact
                task_link_to_contact(db, person_id)?;
            }
            KeyCode::Char('c') => {
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    if task.is_completed() {
                        db.uncomplete_task(task.id)?;
                    } else {
                        db.complete_task(task.id)?;
                    }
                }
            }
            KeyCode::Char('u') => {
                // Unlink: remove person_id from task (doesn't delete)
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    let mut updated = task.clone();
                    updated.person_id = None;
                    db.update_task(&updated)?;
                }
            }
            KeyCode::Char('s') => {
                sort_mode = match sort_mode {
                    SortMode::Quadrant => SortMode::Deadline,
                    SortMode::Deadline => SortMode::Quadrant,
                };
            }
            KeyCode::Enter => {
                if !tasks.is_empty() {
                    let task = &tasks[selected_idx];
                    if let Some(updated) = task_detail_screen(db, task)? {
                        db.update_task(&updated)?;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Add a task linked to a specific contact
pub fn task_add_prompt_for_contact(db: &Database, person_id: Uuid) -> Result<()> {
    let _ = clear_screen();

    // Title (required)
    let title = Text::new("title:")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let Some(title) = title else {
        return Ok(()); // Cancelled
    };

    let title = title.trim();
    if title.is_empty() {
        return Ok(());
    }

    // Quadrant selection
    let quadrant_options = vec![
        "1. Urgent & Important",
        "2. Important",
        "3. Urgent",
        "4. Later (default)",
    ];

    let quadrant_result = Select::new("quadrant:", quadrant_options)
        .with_render_config(minimal_render_config())
        .with_starting_cursor(3)
        .prompt_skippable()?;

    let quadrant = match quadrant_result {
        Some(s) if s.starts_with("1.") => 1,
        Some(s) if s.starts_with("2.") => 2,
        Some(s) if s.starts_with("3.") => 3,
        _ => 4,
    };

    // Deadline (optional)
    let deadline_str = Text::new("deadline (YYYY-MM-DD, optional):")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let deadline = deadline_str.and_then(|s| parse_date_input(&s));

    // Create task linked to this person
    let mut task = Task::new(title.to_string());
    task.quadrant = quadrant;
    task.deadline = deadline;
    task.person_id = Some(person_id);

    db.insert_task(&task)?;
    Ok(())
}

/// Quick add a task linked to a contact (title only, defaults to Q4)
pub fn task_quick_add_for_contact(db: &Database, person_id: Uuid) -> Result<()> {
    let _ = clear_screen();

    let title = Text::new("task:")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let Some(title) = title else {
        return Ok(());
    };

    let title = title.trim();
    if title.is_empty() {
        return Ok(());
    }

    let mut task = Task::new(title.to_string());
    task.quadrant = 4; // Default to Q4 (Later)
    task.person_id = Some(person_id);

    db.insert_task(&task)?;
    Ok(())
}

/// Link an existing unlinked task to a contact
pub fn task_link_to_contact(db: &Database, person_id: Uuid) -> Result<()> {
    let unlinked = db.get_unlinked_pending_tasks()?;

    if unlinked.is_empty() {
        let _ = clear_screen();
        println!("No unlinked tasks available.");
        println!("\nPress any key to continue...");
        let _ = std::io::stdout().flush();
        let _guard = RawModeGuard::new()?;
        let _ = event::read();
        return Ok(());
    }

    let _ = clear_screen();

    // Build selection options
    let options: Vec<String> = unlinked
        .iter()
        .map(|t| {
            let q = format!("Q{}", t.quadrant);
            let title = if t.title.chars().count() > 40 {
                format!("{}...", t.title.chars().take(37).collect::<String>())
            } else {
                t.title.clone()
            };
            format!("{} {}", q, title)
        })
        .collect();

    let selection = Select::new("Link task:", options.clone())
        .with_render_config(minimal_render_config())
        .with_vim_mode(true)
        .prompt_skippable()?;

    if let Some(selected) = selection {
        if let Some(idx) = options.iter().position(|o| *o == selected) {
            let mut task = unlinked[idx].clone();
            task.person_id = Some(person_id);
            db.update_task(&task)?;
        }
    }

    Ok(())
}

/// Task detail screen for viewing/editing a task
/// Returns Some(task) if changes were made, None otherwise
pub fn task_detail_screen(db: &Database, task: &Task) -> Result<Option<Task>> {
    let _ = clear_screen();

    let mut edited = task.clone();
    let mut changed = false;

    println!("Task: {}", task.title);
    println!();

    // Show current values
    println!("  Quadrant: {}", task.quadrant_label());
    if let Some(deadline) = task.deadline {
        println!("  Deadline: {}", format_deadline(deadline));
    }
    if let Some(ref desc) = task.description {
        println!("  Description: {}", desc);
    }
    if task.is_completed() {
        println!("  Status: Completed");
    }

    // Link to person if set
    if let Some(person_id) = task.person_id {
        if let Some(person) = db.get_person_by_id(person_id)? {
            let name = person.display_name.as_deref().unwrap_or("(unnamed)");
            println!("  Linked to: {}", name);
        }
    }

    println!();

    // Edit options
    let options = vec![
        "Edit title",
        "Edit quadrant",
        "Edit deadline",
        "Edit description",
        "Back",
    ];

    loop {
        let selection = Select::new("Action:", options.clone())
            .with_render_config(minimal_render_config())
            .with_starting_cursor(options.len() - 1)
            .prompt_skippable()?;

        let Some(action) = selection else {
            break; // Cancelled
        };

        #[allow(clippy::useless_asref)]
        match action.as_ref() {
            "Edit title" => {
                let new_title = Text::new("title:")
                    .with_render_config(minimal_render_config())
                    .with_default(&edited.title)
                    .prompt_skippable()?;

                if let Some(t) = new_title {
                    let t = t.trim();
                    if !t.is_empty() && t != edited.title {
                        edited.title = t.to_string();
                        edited.updated_at = Utc::now();
                        changed = true;
                    }
                }
            }
            "Edit quadrant" => {
                let quadrant_options = vec![
                    "1. Urgent & Important",
                    "2. Important",
                    "3. Urgent",
                    "4. Later",
                ];

                let quadrant_result = Select::new("quadrant:", quadrant_options)
                    .with_render_config(minimal_render_config())
                    .with_starting_cursor((edited.quadrant - 1).min(3) as usize)
                    .prompt_skippable()?;

                if let Some(s) = quadrant_result {
                    let new_q = if s.starts_with("1.") { 1 }
                    else if s.starts_with("2.") { 2 }
                    else if s.starts_with("3.") { 3 }
                    else { 4 };

                    if new_q != edited.quadrant {
                        edited.quadrant = new_q;
                        edited.updated_at = Utc::now();
                        changed = true;
                    }
                }
            }
            "Edit deadline" => {
                let current = edited.deadline
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();

                let deadline_str = Text::new("deadline (YYYY-MM-DD, empty to clear):")
                    .with_render_config(minimal_render_config())
                    .with_default(&current)
                    .prompt_skippable()?;

                if let Some(s) = deadline_str {
                    let new_deadline = if s.trim().is_empty() {
                        None
                    } else {
                        parse_date_input(&s)
                    };

                    if new_deadline != edited.deadline {
                        edited.deadline = new_deadline;
                        edited.updated_at = Utc::now();
                        changed = true;
                    }
                }
            }
            "Edit description" => {
                let current = edited.description.as_deref().unwrap_or("");
                let desc = Text::new("description:")
                    .with_render_config(minimal_render_config())
                    .with_default(current)
                    .prompt_skippable()?;

                if let Some(d) = desc {
                    let d = d.trim();
                    let new_desc = if d.is_empty() { None } else { Some(d.to_string()) };
                    if new_desc != edited.description {
                        edited.description = new_desc;
                        edited.updated_at = Utc::now();
                        changed = true;
                    }
                }
            }
            "Back" => {
                break;
            }
            _ => {}
        }
    }

    if changed {
        Ok(Some(edited))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_checkbox_display() {
        let task = Task::new("Buy groceries".to_string());
        assert!(!task.is_completed());

        let mut done_task = Task::new("Done task".to_string());
        done_task.complete();
        assert!(done_task.is_completed());
    }

    #[test]
    fn test_quadrant_header() {
        assert_eq!(quadrant_header(1), "Q1: Urgent & Important");
        assert_eq!(quadrant_header(2), "Q2: Important");
        assert_eq!(quadrant_header(3), "Q3: Urgent");
        assert_eq!(quadrant_header(4), "Q4: Later");
        assert_eq!(quadrant_header(99), "Q4: Later");
    }

    #[test]
    fn test_parse_date_input() {
        // Empty returns None
        assert!(parse_date_input("").is_none());
        assert!(parse_date_input("  ").is_none());

        // Valid YYYY-MM-DD
        let date = parse_date_input("2025-03-15");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.date_naive().to_string(), "2025-03-15");

        // Keywords
        assert!(parse_date_input("today").is_some());
        assert!(parse_date_input("tomorrow").is_some());

        // Invalid
        assert!(parse_date_input("not a date").is_none());
    }

    #[test]
    fn test_format_deadline() {
        // Test with a specific date in the future
        let future = Utc::now() + chrono::Duration::days(30);
        let result = format_deadline(future);
        assert!(!result.is_empty());
        assert!(!result.contains("overdue"));
    }

    #[test]
    fn test_task_layout_default() {
        let layout = TaskLayout::default();
        assert_eq!(layout.check_width, 3);
        assert!(layout.title_width >= 30);
    }
}
