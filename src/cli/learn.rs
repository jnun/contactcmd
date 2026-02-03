//! Learn Something - progressive feature discovery CLI

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::io::{self, Write};

use crate::db::learn::LearnableFeature;
use crate::db::Database;
use super::ui::{clear_screen, RawModeGuard, StatusBar};

/// Show a tutorial and wait for user to mark as learned
fn show_tutorial(db: &Database, feature: &LearnableFeature) -> Result<bool> {
    clear_screen()?;

    let t = &feature.tutorial;

    // Header
    println!("{}\n", t.title);
    println!("{}\n", t.summary);

    // Steps
    println!("Steps:");
    for (i, step) in t.steps.iter().enumerate() {
        println!("  {}. {}", i + 1, step);
    }

    // Tips
    if !t.tips.is_empty() {
        println!("\nTips:");
        for tip in &t.tips {
            println!("  - {}", tip);
        }
    }

    // Related
    if !t.related_features.is_empty() {
        println!("\nRelated: {}", t.related_features.join(", "));
    }

    // Progress
    let (learned, total) = db.get_learning_stats()?;
    println!("\nProgress: {}/{} features learned", learned, total);

    // Prompt
    let status = StatusBar::new()
        .action("enter", " got it")
        .action("q", "/esc skip")
        .render();
    print!("\n{}", status);
    io::stdout().flush()?;

    // Wait for input
    let marked = {
        let _guard = RawModeGuard::new()?;
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => break true,
                    KeyCode::Char('q') | KeyCode::Esc => break false,
                    _ => {}
                }
            }
        }
    };

    if marked {
        db.mark_feature_learned(&feature.id)?;
        println!("\r\nMarked as learned.");
    }

    Ok(marked)
}

/// Run the learn command
pub fn run_learn(db: &Database, query: Option<&str>) -> Result<()> {
    let feature = match query {
        Some(q) if !q.is_empty() => db.find_feature_by_name(q)?,
        _ => db.get_next_to_learn()?,
    };

    match feature {
        Some(f) => {
            show_tutorial(db, &f)?;
        }
        None => {
            if query.is_some() {
                println!("No feature found matching that query.");
            } else {
                println!("No tutorials available.");
            }
        }
    }

    Ok(())
}

/// Show learning progress
pub fn run_learn_progress(db: &Database) -> Result<()> {
    let (learned, total) = db.get_learning_stats()?;
    let all_done = db.all_features_learned_once()?;

    println!("Learning progress: {}/{}", learned, total);

    if all_done {
        println!("You've seen all features at least once.");
    } else {
        let remaining = total - learned;
        println!("{} features remaining.", remaining);
    }

    Ok(())
}
