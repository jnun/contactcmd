use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

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
    }
}
