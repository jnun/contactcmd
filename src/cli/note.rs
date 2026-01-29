//! Quick note command for adding timestamped notes to contacts

use anyhow::Result;
use chrono::{Local, Utc};
use inquire::{Select, Text};

use crate::cli::ui::{minimal_render_config, visible_lines};
use crate::db::Database;
use crate::models::Person;

/// Execute the note command
pub fn run_note(db: &Database, search: &str, note_text: Option<String>) -> Result<()> {
    let search = search.trim();
    if search.is_empty() {
        println!("No search term.");
        return Ok(());
    }

    // Search for contacts
    let words: Vec<&str> = search.split_whitespace().collect();
    let results = db.search_persons_multi(&words, false, 20)?;

    match results.len() {
        0 => {
            println!("No matches.");
        }
        1 => {
            add_note_to_person(db, &results[0], note_text)?;
        }
        _ => {
            select_and_add_note(db, &results, note_text)?;
        }
    }

    Ok(())
}

fn select_and_add_note(db: &Database, results: &[Person], note_text: Option<String>) -> Result<()> {
    // Build selection options
    let person_ids: Vec<_> = results.iter().map(|p| p.id).collect();
    let display_info = db.get_display_info_for_persons(&person_ids)?;

    let options: Vec<String> = results
        .iter()
        .map(|p| {
            let name = p.display_name.as_deref().unwrap_or("?");
            let (email, _) = display_info.get(&p.id).cloned().unwrap_or((None, None));
            match email {
                Some(e) => format!("{} ({})", name, e),
                None => name.to_string(),
            }
        })
        .collect();

    let selection = Select::new("", options.clone())
        .with_render_config(minimal_render_config())
        .with_page_size(visible_lines())
        .with_vim_mode(true)
        .prompt_skippable()?;

    let Some(selected) = selection else {
        return Ok(());
    };

    // Find the selected person
    let idx = options.iter().position(|o| o == &selected).unwrap_or(0);
    add_note_to_person(db, &results[idx], note_text)?;

    Ok(())
}

fn add_note_to_person(db: &Database, person: &Person, note_text: Option<String>) -> Result<()> {
    // Get note text - either from argument or prompt
    let note = match note_text {
        Some(text) if !text.is_empty() => text,
        _ => {
            let input = Text::new("note:")
                .with_render_config(minimal_render_config())
                .prompt_skippable()?;
            match input {
                Some(t) if !t.is_empty() => t,
                _ => return Ok(()),
            }
        }
    };

    // Format timestamp in local time with timezone
    let now = Utc::now();
    let local = now.with_timezone(&Local);
    let tz = local.format("%Z").to_string();
    let timestamp = format!("[{} {}]", local.format("%Y-%m-%d %H:%M"), tz);
    let timestamped_note = format!("{} {}", timestamp, note);

    // Append to existing notes
    let mut updated_person = person.clone();
    updated_person.notes = match &person.notes {
        Some(existing) if !existing.is_empty() => {
            Some(format!("{}\n{}", existing, timestamped_note))
        }
        _ => Some(timestamped_note),
    };

    db.update_person(&updated_person)?;
    println!("Saved.");

    Ok(())
}
