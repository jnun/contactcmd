//! Learn Something feature - progressive feature discovery
//!
//! Tracks which features the user has learned and provides tutorials
//! in a spaced-repetition style (lowest times_learned gets picked first).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Database;

/// Tutorial content stored as JSON in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tutorial {
    pub title: String,
    pub summary: String,
    pub steps: Vec<String>,
    pub tips: Vec<String>,
    pub related_features: Vec<String>,
}

/// A learnable feature with its tutorial and progress
#[derive(Debug, Clone)]
pub struct LearnableFeature {
    pub id: String,
    pub feature_name: String,
    pub category: String,
    pub tutorial: Tutorial,
    pub times_learned: i32,
}

impl Database {
    /// Get the next feature to learn (lowest times_learned value)
    /// Returns None if no features exist
    pub fn get_next_to_learn(&self) -> Result<Option<LearnableFeature>> {
        let result = self.conn.query_row(
            "SELECT id, feature_name, category, tutorial_json, times_learned
             FROM learn_something
             ORDER BY times_learned ASC, created_at ASC
             LIMIT 1",
            [],
            |row| {
                let tutorial_json: String = row.get(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    tutorial_json,
                    row.get::<_, i32>(4)?,
                ))
            },
        );

        match result {
            Ok((id, feature_name, category, tutorial_json, times_learned)) => {
                let tutorial: Tutorial = serde_json::from_str(&tutorial_json)
                    .map_err(|e| anyhow::anyhow!("Invalid tutorial JSON: {}", e))?;
                Ok(Some(LearnableFeature {
                    id,
                    feature_name,
                    category,
                    tutorial,
                    times_learned,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Mark a feature as learned (increment times_learned)
    pub fn mark_feature_learned(&self, feature_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE learn_something SET times_learned = times_learned + 1 WHERE id = ?",
            [feature_id],
        )?;
        Ok(())
    }

    /// Search for a feature by name (for AI "teach me about X" queries)
    pub fn find_feature_by_name(&self, query: &str) -> Result<Option<LearnableFeature>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let result = self.conn.query_row(
            "SELECT id, feature_name, category, tutorial_json, times_learned
             FROM learn_something
             WHERE LOWER(feature_name) LIKE ? OR LOWER(category) LIKE ?
             ORDER BY times_learned ASC
             LIMIT 1",
            [&search_pattern, &search_pattern],
            |row| {
                let tutorial_json: String = row.get(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    tutorial_json,
                    row.get::<_, i32>(4)?,
                ))
            },
        );

        match result {
            Ok((id, feature_name, category, tutorial_json, times_learned)) => {
                let tutorial: Tutorial = serde_json::from_str(&tutorial_json)
                    .map_err(|e| anyhow::anyhow!("Invalid tutorial JSON: {}", e))?;
                Ok(Some(LearnableFeature {
                    id,
                    feature_name,
                    category,
                    tutorial,
                    times_learned,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if all features have been learned at least once (refresher mode)
    pub fn all_features_learned_once(&self) -> Result<bool> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM learn_something WHERE times_learned = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Get learning progress stats
    pub fn get_learning_stats(&self) -> Result<(i32, i32)> {
        let total: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM learn_something",
            [],
            |row| row.get(0),
        )?;
        let learned: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM learn_something WHERE times_learned > 0",
            [],
            |row| row.get(0),
        )?;
        Ok((learned, total))
    }

    /// Seed initial tutorials (called during migration)
    pub(crate) fn seed_learn_something(&self) -> Result<()> {
        let tutorials = get_seed_tutorials();

        for (feature_name, category, tutorial) in tutorials {
            let id = Uuid::new_v4().to_string();
            let tutorial_json = serde_json::to_string(&tutorial)?;

            self.conn.execute(
                "INSERT OR IGNORE INTO learn_something (id, feature_name, category, tutorial_json)
                 VALUES (?, ?, ?, ?)",
                rusqlite::params![id, feature_name, category, tutorial_json],
            )?;
        }

        Ok(())
    }
}

/// Get seed tutorials for initial database population
fn get_seed_tutorials() -> Vec<(&'static str, &'static str, Tutorial)> {
    vec![
        // Search & Navigation
        (
            "Search Contacts",
            "navigation",
            Tutorial {
                title: "Search Contacts".to_string(),
                summary: "Find contacts quickly by name, email, phone, or organization.".to_string(),
                steps: vec![
                    "Type /search followed by your query (e.g., /search john)".to_string(),
                    "Search matches names, emails, phone numbers, and organizations".to_string(),
                    "Use multiple words to narrow results (e.g., /search john google)".to_string(),
                    "Results appear as a numbered list".to_string(),
                ],
                tips: vec![
                    "Use /s as a shortcut for /search".to_string(),
                    "Single matches open directly in card view".to_string(),
                ],
                related_features: vec!["Browse Results".to_string(), "List Contacts".to_string()],
            },
        ),
        (
            "Browse Results",
            "navigation",
            Tutorial {
                title: "Browse Results".to_string(),
                summary: "View and interact with search results in a full-screen card view.".to_string(),
                steps: vec![
                    "After a search, type /browse (or /b) to enter browse mode".to_string(),
                    "Use arrow keys to navigate between contacts".to_string(),
                    "Press Enter to view full contact details".to_string(),
                    "Press Escape to return to chat".to_string(),
                ],
                tips: vec![
                    "Browse mode shows contact photos when available".to_string(),
                    "You can take actions on contacts from browse view".to_string(),
                ],
                related_features: vec!["Search Contacts".to_string()],
            },
        ),
        (
            "List All Contacts",
            "navigation",
            Tutorial {
                title: "List All Contacts".to_string(),
                summary: "View all your contacts at once.".to_string(),
                steps: vec![
                    "Type /list (or /l) to see all contacts".to_string(),
                    "Contacts are shown with their primary email and organization".to_string(),
                    "Use /browse after listing to view in card mode".to_string(),
                ],
                tips: vec![
                    "For large contact lists, use /search to narrow down first".to_string(),
                ],
                related_features: vec!["Search Contacts".to_string(), "Browse Results".to_string()],
            },
        ),
        // Messages
        (
            "View Messages",
            "messaging",
            Tutorial {
                title: "View Messages".to_string(),
                summary: "See your iMessage/SMS conversation history with a contact.".to_string(),
                steps: vec![
                    "Type /messages followed by a contact name (e.g., /m alice)".to_string(),
                    "The app finds the contact and shows message history".to_string(),
                    "Messages are displayed in chronological order".to_string(),
                    "Press Escape to return to chat".to_string(),
                ],
                tips: vec![
                    "This reads from your Mac's Messages database (read-only)".to_string(),
                    "Use /m as a shortcut for /messages".to_string(),
                ],
                related_features: vec!["Search Contacts".to_string(), "Recent Contacts".to_string()],
            },
        ),
        (
            "Recent Contacts",
            "messaging",
            Tutorial {
                title: "Recent Contacts".to_string(),
                summary: "See contacts you've texted recently via iMessage or SMS.".to_string(),
                steps: vec![
                    "Type /recent (or /r) to see contacts from the last 7 days".to_string(),
                    "Optionally specify days: /recent 30 for last 30 days".to_string(),
                    "Each contact shows time since last message and service type".to_string(),
                    "Use /browse to view details of matched contacts".to_string(),
                ],
                tips: vec![
                    "Unknown numbers appear with '(unknown)' - consider adding them".to_string(),
                    "Requires Full Disk Access permission in System Settings".to_string(),
                ],
                related_features: vec!["View Messages".to_string(), "Browse Results".to_string()],
            },
        ),
        // Data Management
        (
            "Import Contacts",
            "data",
            Tutorial {
                title: "Import Contacts".to_string(),
                summary: "Import contacts from a CSV file.".to_string(),
                steps: vec![
                    "Type /import to start the import wizard".to_string(),
                    "Select your CSV file using the file picker".to_string(),
                    "Map CSV columns to contact fields".to_string(),
                    "Review and confirm the import".to_string(),
                ],
                tips: vec![
                    "CSV should have headers in the first row".to_string(),
                    "Common formats (Google, LinkedIn) are auto-detected".to_string(),
                ],
                related_features: vec!["Sync Contacts".to_string()],
            },
        ),
        (
            "Sync Contacts",
            "data",
            Tutorial {
                title: "Sync Contacts".to_string(),
                summary: "Synchronize with your Mac's Contacts app.".to_string(),
                steps: vec![
                    "Type /sync to start synchronization".to_string(),
                    "The app reads from your Mac's Contacts database".to_string(),
                    "New contacts are added, existing ones are updated".to_string(),
                    "Photos are synced when available".to_string(),
                ],
                tips: vec![
                    "Sync is one-way (Mac Contacts to this app)".to_string(),
                    "Run sync periodically to stay up to date".to_string(),
                ],
                related_features: vec!["Import Contacts".to_string()],
            },
        ),
        // AI Features
        (
            "AI Chat",
            "ai",
            Tutorial {
                title: "AI Chat".to_string(),
                summary: "Use natural language to interact with your contacts.".to_string(),
                steps: vec![
                    "Type naturally without a / prefix to chat with the AI".to_string(),
                    "Ask things like 'find contacts at Google'".to_string(),
                    "Or 'show me Alice's details'".to_string(),
                    "The AI translates your request into commands".to_string(),
                ],
                tips: vec![
                    "AI needs to be configured first (run /setup)".to_string(),
                    "The AI has no direct access to your data - it only suggests commands".to_string(),
                ],
                related_features: vec!["Setup".to_string()],
            },
        ),
        // Settings
        (
            "Setup & Configuration",
            "settings",
            Tutorial {
                title: "Setup & Configuration".to_string(),
                summary: "Configure AI, email, and other app settings.".to_string(),
                steps: vec![
                    "Type /setup to open the configuration wizard".to_string(),
                    "Set your AI provider and API key for AI features".to_string(),
                    "Configure email settings for contact communication".to_string(),
                    "Settings are stored securely in your local database".to_string(),
                ],
                tips: vec![
                    "You can run /setup anytime to change settings".to_string(),
                    "API keys are stored locally, never sent elsewhere".to_string(),
                ],
                related_features: vec!["AI Chat".to_string()],
            },
        ),
        // UI Modes
        (
            "Keyboard Shortcuts",
            "ui",
            Tutorial {
                title: "Keyboard Shortcuts".to_string(),
                summary: "Navigate efficiently with keyboard shortcuts.".to_string(),
                steps: vec![
                    "Up/Down arrows: Navigate command history".to_string(),
                    "Ctrl+L: Clear the screen".to_string(),
                    "Ctrl+C or Escape: Cancel current action".to_string(),
                    "Tab: Auto-complete (where available)".to_string(),
                ],
                tips: vec![
                    "Most commands have single-letter shortcuts (e.g., /s for /search)".to_string(),
                    "Type 'help' or '/help' to see all commands".to_string(),
                ],
                related_features: vec![],
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_and_query() {
        let db = Database::open_memory().unwrap();

        // Should have seeded tutorials
        let (learned, total) = db.get_learning_stats().unwrap();
        assert!(total > 0, "Should have seeded tutorials");
        assert_eq!(learned, 0, "None should be learned yet");
    }

    #[test]
    fn test_get_next_to_learn() {
        let db = Database::open_memory().unwrap();

        let feature = db.get_next_to_learn().unwrap();
        assert!(feature.is_some(), "Should return a feature");

        let feature = feature.unwrap();
        assert_eq!(feature.times_learned, 0);
    }

    #[test]
    fn test_mark_learned() {
        let db = Database::open_memory().unwrap();

        let feature = db.get_next_to_learn().unwrap().unwrap();
        let id = feature.id.clone();

        db.mark_feature_learned(&id).unwrap();

        // Get the same feature again - it should have times_learned = 1
        let updated: i32 = db.conn.query_row(
            "SELECT times_learned FROM learn_something WHERE id = ?",
            [&id],
            |row| row.get(0),
        ).unwrap();

        assert_eq!(updated, 1);
    }

    #[test]
    fn test_find_by_name() {
        let db = Database::open_memory().unwrap();

        let feature = db.find_feature_by_name("search").unwrap();
        assert!(feature.is_some());
        assert!(feature.unwrap().feature_name.to_lowercase().contains("search"));
    }

    #[test]
    fn test_all_learned_once() {
        let db = Database::open_memory().unwrap();

        // Initially not all learned
        assert!(!db.all_features_learned_once().unwrap());
    }
}
