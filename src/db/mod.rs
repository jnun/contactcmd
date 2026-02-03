use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

pub mod gateway;
pub mod learn;
mod persons;
mod schema;

pub use schema::SCHEMA_VERSION;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open database, creating if needed, running migrations
    pub fn open() -> Result<Self> {
        let path = Self::default_path()?;
        Self::open_at(path)
    }

    pub fn open_at(path: PathBuf) -> Result<Self> {
        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            // Also create photos directory
            let photos_dir = parent.join("photos");
            std::fs::create_dir_all(&photos_dir)?;
        }

        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Get the path to the photos directory
    pub fn photos_dir() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join("contactcmd").join("photos"))
    }

    /// Open in-memory database for testing
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    #[allow(dead_code)]
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    fn default_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join("contactcmd").join("contacts.db"))
    }

    fn migrate(&self) -> Result<()> {
        let version = self.get_schema_version()?;

        if version == 0 {
            // Run migration in a transaction for atomicity
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::SCHEMA_V1))?;
            self.set_schema_version(1)?;
        }

        if self.get_schema_version()? == 1 {
            // V1 → V2: Add photo_path column
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V2))?;
            self.set_schema_version(2)?;
        }

        if self.get_schema_version()? == 2 {
            // V2 → V3: Drop photo_path column (photos now derived from UUID)
            self.migrate_v3()?;
            self.set_schema_version(3)?;
        }

        if self.get_schema_version()? == 3 {
            // V3 → V4: Add app_settings table
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V4))?;
            self.set_schema_version(4)?;
        }

        if self.get_schema_version()? == 4 {
            // V4 → V5: Add oauth_tokens table
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V5))?;
            self.set_schema_version(5)?;
        }

        if self.get_schema_version()? == 5 {
            // V5 → V6: Add tasks table
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V6))?;
            self.set_schema_version(6)?;
        }

        if self.get_schema_version()? == 6 {
            // V6 → V7: Add gateway tables (api_keys + communication_queue)
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V7))?;
            self.set_schema_version(7)?;
        }

        if self.get_schema_version()? == 7 {
            // V7 → V8: Add checkin_date column to persons
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V8))?;
            self.set_schema_version(8)?;
        }

        if self.get_schema_version()? == 8 {
            // V8 → V9: Add learn_something table for progressive feature discovery
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V9))?;
            self.set_schema_version(9)?;
            // Seed initial tutorials
            self.seed_learn_something()?;
        }

        if self.get_schema_version()? == 9 {
            // V9 → V10: Add rate limiting columns to api_keys
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V10))?;
            self.set_schema_version(10)?;
        }

        if self.get_schema_version()? == 10 {
            // V10 → V11: Add recipient allowlist table
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V11))?;
            self.set_schema_version(11)?;
        }

        if self.get_schema_version()? == 11 {
            // V11 → V12: Add content_filters table for message safety screening
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V12))?;
            self.set_schema_version(12)?;
            // Seed default content filters
            self.seed_content_filters()?;
        }

        if self.get_schema_version()? == 12 {
            // V12 → V13: Add webhook_url column to api_keys
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V13))?;
            self.set_schema_version(13)?;
        }

        if self.get_schema_version()? == 13 {
            // V13 → V14: Add ai_contact_allowed column to persons
            self.conn
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema::MIGRATION_V14))?;
            self.set_schema_version(14)?;
        }

        Ok(())
    }

    /// V3 migration: Drop photo_path column
    /// Tries DROP COLUMN first (SQLite 3.35.0+), falls back to table rebuild
    fn migrate_v3(&self) -> Result<()> {
        // First, check if photo_path column exists (it might not on fresh V3 installs)
        let has_photo_path = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('persons') WHERE name = 'photo_path'",
            [],
            |row| row.get::<_, i32>(0),
        )? > 0;

        if !has_photo_path {
            // Column doesn't exist, nothing to do
            return Ok(());
        }

        // Try DROP COLUMN first (SQLite 3.35.0+)
        let drop_result = self.conn.execute_batch(&format!(
            "BEGIN TRANSACTION; {} COMMIT;",
            schema::MIGRATION_V3_DROP_COLUMN
        ));

        if drop_result.is_ok() {
            return Ok(());
        }

        // Fallback: rebuild table
        self.conn.execute_batch(&format!(
            "BEGIN TRANSACTION; {} COMMIT;",
            schema::MIGRATION_V3_REBUILD
        ))?;

        Ok(())
    }

    fn get_schema_version(&self) -> Result<i32> {
        let result: Result<i32, _> =
            self.conn
                .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                    row.get(0)
                });

        match result {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(rusqlite::Error::SqliteFailure(err, msg)) => {
                // "no such table" is error code 1 (SQLITE_ERROR)
                if err.code == rusqlite::ErrorCode::Unknown
                    && msg.as_ref().is_some_and(|m| m.contains("no such table"))
                {
                    Ok(0)
                } else {
                    Err(rusqlite::Error::SqliteFailure(err, msg).into())
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    fn set_schema_version(&self, version: i32) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (id, version) VALUES (1, ?)",
            [version],
        )?;
        Ok(())
    }

    // App settings methods

    /// Get an app setting by key
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?",
            [key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set an app setting (insert or update)
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Delete an app setting
    pub fn delete_setting(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM app_settings WHERE key = ?", [key])?;
        Ok(())
    }

    // OAuth token methods

    /// Store OAuth tokens for a provider
    pub fn save_oauth_token(
        &self,
        provider: &str,
        email: &str,
        refresh_token: &str,
        access_token: Option<&str>,
        expires_at: Option<i64>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO oauth_tokens (provider, email, refresh_token, access_token, expires_at)
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![provider, email, refresh_token, access_token, expires_at],
        )?;
        Ok(())
    }

    /// Get OAuth tokens for a provider
    pub fn get_oauth_token(&self, provider: &str) -> Result<Option<OAuthToken>> {
        let result = self.conn.query_row(
            "SELECT email, refresh_token, access_token, expires_at FROM oauth_tokens WHERE provider = ?",
            [provider],
            |row| {
                Ok(OAuthToken {
                    email: row.get(0)?,
                    refresh_token: row.get(1)?,
                    access_token: row.get(2)?,
                    expires_at: row.get(3)?,
                })
            },
        );

        match result {
            Ok(token) => Ok(Some(token)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update access token and expiry for a provider
    pub fn update_oauth_access_token(
        &self,
        provider: &str,
        access_token: &str,
        expires_at: i64,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE oauth_tokens SET access_token = ?, expires_at = ? WHERE provider = ?",
            rusqlite::params![access_token, expires_at, provider],
        )?;
        Ok(())
    }

    /// Delete OAuth tokens for a provider
    pub fn delete_oauth_token(&self, provider: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM oauth_tokens WHERE provider = ?", [provider])?;
        Ok(())
    }
}

/// OAuth token data
#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub email: String,
    pub refresh_token: String,
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory() {
        let db = Database::open_memory().unwrap();
        assert_eq!(db.get_schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_tables_exist() {
        let db = Database::open_memory().unwrap();

        let tables: Vec<String> = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"persons".to_string()));
        assert!(tables.contains(&"emails".to_string()));
        assert!(tables.contains(&"phones".to_string()));
        assert!(tables.contains(&"addresses".to_string()));
        assert!(tables.contains(&"organizations".to_string()));
        assert!(tables.contains(&"person_organizations".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"person_tags".to_string()));
        assert!(tables.contains(&"special_dates".to_string()));
        assert!(tables.contains(&"notes".to_string()));
        assert!(tables.contains(&"interactions".to_string()));
        assert!(tables.contains(&"app_settings".to_string()));
        assert!(tables.contains(&"content_filters".to_string()));
    }

    #[test]
    fn test_app_settings() {
        let db = Database::open_memory().unwrap();

        // Initially no setting
        assert!(db.get_setting("email_account").unwrap().is_none());

        // Set a setting
        db.set_setting("email_account", "test@example.com").unwrap();
        assert_eq!(
            db.get_setting("email_account").unwrap(),
            Some("test@example.com".to_string())
        );

        // Update a setting
        db.set_setting("email_account", "new@example.com").unwrap();
        assert_eq!(
            db.get_setting("email_account").unwrap(),
            Some("new@example.com".to_string())
        );

        // Delete a setting
        db.delete_setting("email_account").unwrap();
        assert!(db.get_setting("email_account").unwrap().is_none());
    }
}
