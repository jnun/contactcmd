//! Database operations for the communication gateway.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::Database;

/// API key stored in database
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub rate_limit_per_hour: i32,
    pub rate_limit_per_day: i32,
    pub webhook_url: Option<String>,
}

/// Recipient allowlist entry for an API key
#[derive(Debug, Clone)]
pub struct AllowlistEntry {
    pub id: String,
    pub api_key_id: String,
    pub recipient_pattern: String,
    pub created_at: DateTime<Utc>,
}

/// Content filter for message safety screening
#[derive(Debug, Clone)]
pub struct ContentFilter {
    pub id: String,
    pub pattern: String,
    pub pattern_type: String, // 'regex' or 'literal'
    pub action: String,       // 'deny' or 'flag'
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Communication queue entry
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: String,
    pub api_key_id: String,
    pub channel: String,
    pub recipient_address: String,
    pub recipient_name: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub priority: String,
    pub status: String,
    pub agent_context: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl Database {
    // ========== API Key Operations ==========

    /// Insert a new API key
    pub fn insert_api_key(
        &self,
        id: &str,
        name: &str,
        key_hash: &str,
        key_prefix: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO api_keys (id, name, key_hash, key_prefix, created_at) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![id, name, key_hash, key_prefix, now],
        )?;
        Ok(())
    }

    /// Find API key by hash (for authentication)
    pub fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
        let result = self.conn.query_row(
            "SELECT id, name, key_hash, key_prefix, created_at, last_used_at, revoked_at,
                    rate_limit_per_hour, rate_limit_per_day, webhook_url
             FROM api_keys WHERE key_hash = ?",
            [key_hash],
            |row| {
                Ok(ApiKey {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    key_hash: row.get(2)?,
                    key_prefix: row.get(3)?,
                    created_at: parse_datetime(row.get::<_, String>(4)?),
                    last_used_at: row.get::<_, Option<String>>(5)?.map(parse_datetime),
                    revoked_at: row.get::<_, Option<String>>(6)?.map(parse_datetime),
                    rate_limit_per_hour: row.get(7)?,
                    rate_limit_per_day: row.get(8)?,
                    webhook_url: row.get(9)?,
                })
            },
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all API keys (for management)
    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, key_hash, key_prefix, created_at, last_used_at, revoked_at,
                    rate_limit_per_hour, rate_limit_per_day, webhook_url
             FROM api_keys ORDER BY created_at DESC",
        )?;

        let keys = stmt
            .query_map([], |row| {
                Ok(ApiKey {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    key_hash: row.get(2)?,
                    key_prefix: row.get(3)?,
                    created_at: parse_datetime(row.get::<_, String>(4)?),
                    last_used_at: row.get::<_, Option<String>>(5)?.map(parse_datetime),
                    revoked_at: row.get::<_, Option<String>>(6)?.map(parse_datetime),
                    rate_limit_per_hour: row.get(7)?,
                    rate_limit_per_day: row.get(8)?,
                    webhook_url: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(keys)
    }

    /// Update last_used_at for an API key
    pub fn touch_api_key(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE api_keys SET last_used_at = ? WHERE id = ?",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    /// Revoke an API key
    pub fn revoke_api_key(&self, id: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE api_keys SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
            rusqlite::params![now, id],
        )?;
        Ok(rows > 0)
    }

    /// Set webhook URL for an API key
    pub fn set_api_key_webhook(&self, id: &str, webhook_url: Option<&str>) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE api_keys SET webhook_url = ? WHERE id = ?",
            rusqlite::params![webhook_url, id],
        )?;
        Ok(rows > 0)
    }

    /// Get webhook URL for an API key by ID
    pub fn get_api_key_webhook(&self, id: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT webhook_url FROM api_keys WHERE id = ?",
            [id],
            |row| row.get::<_, Option<String>>(0),
        );

        match result {
            Ok(url) => Ok(url),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ========== Allowlist Operations ==========

    /// Add a recipient pattern to an API key's allowlist
    /// Returns Ok(true) if inserted, Ok(false) if already exists (idempotent)
    pub fn insert_allowlist_entry(
        &self,
        id: &str,
        api_key_id: &str,
        recipient_pattern: &str,
    ) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let result = self.conn.execute(
            "INSERT OR IGNORE INTO api_key_allowlists (id, api_key_id, recipient_pattern, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![id, api_key_id, recipient_pattern, now],
        );

        match result {
            Ok(rows) => Ok(rows > 0),
            Err(e) => Err(e.into()),
        }
    }

    /// List all allowlist entries for an API key
    pub fn list_allowlist_entries(&self, api_key_id: &str) -> Result<Vec<AllowlistEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, api_key_id, recipient_pattern, created_at
             FROM api_key_allowlists
             WHERE api_key_id = ?
             ORDER BY created_at ASC",
        )?;

        let entries = stmt
            .query_map([api_key_id], |row| {
                Ok(AllowlistEntry {
                    id: row.get(0)?,
                    api_key_id: row.get(1)?,
                    recipient_pattern: row.get(2)?,
                    created_at: parse_datetime(row.get::<_, String>(3)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Delete an allowlist entry by pattern
    pub fn delete_allowlist_entry(&self, api_key_id: &str, recipient_pattern: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM api_key_allowlists WHERE api_key_id = ? AND recipient_pattern = ?",
            rusqlite::params![api_key_id, recipient_pattern],
        )?;
        Ok(rows > 0)
    }

    /// Check if an API key has any allowlist entries (for enforcement)
    pub fn has_allowlist(&self, api_key_id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM api_key_allowlists WHERE api_key_id = ?",
            [api_key_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ========== Communication Queue Operations ==========

    /// Insert a new queue entry
    pub fn insert_queue_entry(
        &self,
        id: &str,
        api_key_id: &str,
        channel: &str,
        recipient_address: &str,
        recipient_name: Option<&str>,
        subject: Option<&str>,
        body: &str,
        priority: &str,
        agent_context: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO communication_queue
             (id, api_key_id, channel, recipient_address, recipient_name, subject, body, priority, status, agent_context, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
            rusqlite::params![id, api_key_id, channel, recipient_address, recipient_name, subject, body, priority, agent_context, now],
        )?;
        Ok(())
    }

    /// Get a queue entry by ID
    pub fn get_queue_entry(&self, id: &str) -> Result<Option<QueueEntry>> {
        let result = self.conn.query_row(
            "SELECT id, api_key_id, channel, recipient_address, recipient_name, subject, body,
                    priority, status, agent_context, created_at, reviewed_at, sent_at, error_message
             FROM communication_queue WHERE id = ?",
            [id],
            row_to_queue_entry,
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List pending queue entries (includes flagged entries that need review)
    pub fn list_pending_queue(&self) -> Result<Vec<QueueEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, api_key_id, channel, recipient_address, recipient_name, subject, body,
                    priority, status, agent_context, created_at, reviewed_at, sent_at, error_message
             FROM communication_queue
             WHERE status IN ('pending', 'flagged')
             ORDER BY
                CASE status WHEN 'flagged' THEN 0 ELSE 1 END,
                CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
                created_at ASC",
        )?;

        let entries = stmt
            .query_map([], row_to_queue_entry)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Count pending queue entries (includes flagged entries that need review)
    pub fn count_pending_queue(&self) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM communication_queue WHERE status IN ('pending', 'flagged')",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count messages queued by an API key within a time window (for rate limiting)
    pub fn count_queue_since(&self, api_key_id: &str, since: DateTime<Utc>) -> Result<i64> {
        let since_str = since.to_rfc3339();
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM communication_queue
             WHERE api_key_id = ? AND created_at >= ?",
            rusqlite::params![api_key_id, since_str],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// List queue entries with optional filters for history/audit log
    pub fn list_queue_history(
        &self,
        status_filter: Option<&str>,
        agent_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(QueueEntry, String)>> {
        // Build query with optional filters
        let mut sql = String::from(
            "SELECT q.id, q.api_key_id, q.channel, q.recipient_address, q.recipient_name,
                    q.subject, q.body, q.priority, q.status, q.agent_context,
                    q.created_at, q.reviewed_at, q.sent_at, q.error_message,
                    k.name as agent_name
             FROM communication_queue q
             LEFT JOIN api_keys k ON q.api_key_id = k.id
             WHERE 1=1",
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(status) = status_filter {
            sql.push_str(" AND q.status = ?");
            params.push(Box::new(status.to_string()));
        }

        if let Some(agent) = agent_filter {
            sql.push_str(" AND k.name LIKE ?");
            params.push(Box::new(format!("%{}%", agent)));
        }

        sql.push_str(" ORDER BY q.created_at DESC LIMIT ?");
        params.push(Box::new(limit as i64));

        let mut stmt = self.conn.prepare(&sql)?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt
            .query_map(param_refs.as_slice(), |row| {
                let entry = QueueEntry {
                    id: row.get(0)?,
                    api_key_id: row.get(1)?,
                    channel: row.get(2)?,
                    recipient_address: row.get(3)?,
                    recipient_name: row.get(4)?,
                    subject: row.get(5)?,
                    body: row.get(6)?,
                    priority: row.get(7)?,
                    status: row.get(8)?,
                    agent_context: row.get(9)?,
                    created_at: parse_datetime(row.get::<_, String>(10)?),
                    reviewed_at: row.get::<_, Option<String>>(11)?.map(parse_datetime),
                    sent_at: row.get::<_, Option<String>>(12)?.map(parse_datetime),
                    error_message: row.get(13)?,
                };
                let agent_name: String = row.get::<_, Option<String>>(14)?.unwrap_or_else(|| "unknown".to_string());
                Ok((entry, agent_name))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Update queue entry status
    pub fn update_queue_status(&self, id: &str, status: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE communication_queue SET status = ?, reviewed_at = ? WHERE id = ?",
            rusqlite::params![status, now, id],
        )?;
        Ok(rows > 0)
    }

    /// Mark queue entry as sent
    pub fn mark_queue_sent(&self, id: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE communication_queue SET status = 'sent', sent_at = ? WHERE id = ?",
            rusqlite::params![now, id],
        )?;
        Ok(rows > 0)
    }

    /// Mark queue entry as failed
    pub fn mark_queue_failed(&self, id: &str, error: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE communication_queue SET status = 'failed', sent_at = ?, error_message = ? WHERE id = ?",
            rusqlite::params![now, error, id],
        )?;
        Ok(rows > 0)
    }

    // ========== Content Filter Operations ==========

    /// Insert a new content filter
    pub fn insert_content_filter(
        &self,
        id: &str,
        pattern: &str,
        pattern_type: &str,
        action: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO content_filters (id, pattern, pattern_type, action, description, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, 1, ?)",
            rusqlite::params![id, pattern, pattern_type, action, description, now],
        )?;
        Ok(())
    }

    /// List all content filters
    pub fn list_content_filters(&self) -> Result<Vec<ContentFilter>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern, pattern_type, action, description, enabled, created_at
             FROM content_filters
             ORDER BY created_at ASC",
        )?;

        let filters = stmt
            .query_map([], |row| {
                Ok(ContentFilter {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    pattern_type: row.get(2)?,
                    action: row.get(3)?,
                    description: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    created_at: parse_datetime(row.get::<_, String>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(filters)
    }

    /// List enabled content filters (for enforcement)
    pub fn list_enabled_content_filters(&self) -> Result<Vec<ContentFilter>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern, pattern_type, action, description, enabled, created_at
             FROM content_filters
             WHERE enabled = 1
             ORDER BY action DESC, created_at ASC",
        )?;

        let filters = stmt
            .query_map([], |row| {
                Ok(ContentFilter {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    pattern_type: row.get(2)?,
                    action: row.get(3)?,
                    description: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    created_at: parse_datetime(row.get::<_, String>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(filters)
    }

    /// Get a content filter by ID
    pub fn get_content_filter(&self, id: &str) -> Result<Option<ContentFilter>> {
        let result = self.conn.query_row(
            "SELECT id, pattern, pattern_type, action, description, enabled, created_at
             FROM content_filters WHERE id = ?",
            [id],
            |row| {
                Ok(ContentFilter {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    pattern_type: row.get(2)?,
                    action: row.get(3)?,
                    description: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    created_at: parse_datetime(row.get::<_, String>(6)?),
                })
            },
        );

        match result {
            Ok(filter) => Ok(Some(filter)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Enable or disable a content filter
    pub fn set_content_filter_enabled(&self, id: &str, enabled: bool) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE content_filters SET enabled = ? WHERE id = ?",
            rusqlite::params![if enabled { 1 } else { 0 }, id],
        )?;
        Ok(rows > 0)
    }

    /// Delete a content filter
    pub fn delete_content_filter(&self, id: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM content_filters WHERE id = ?",
            [id],
        )?;
        Ok(rows > 0)
    }

    /// Seed default content filters (called during migration)
    pub(crate) fn seed_content_filters(&self) -> Result<()> {
        let default_filters = get_default_content_filters();

        for (pattern, pattern_type, action, description) in default_filters {
            let id = Uuid::new_v4().to_string();
            self.insert_content_filter(&id, pattern, pattern_type, action, Some(description))?;
        }

        Ok(())
    }
}

/// Default content filters for message safety
/// Returns: (pattern, pattern_type, action, description)
fn get_default_content_filters() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // SSN pattern (XXX-XX-XXXX format)
        (
            r"\b\d{3}-\d{2}-\d{4}\b",
            "regex",
            "deny",
            "Social Security Number pattern (XXX-XX-XXXX)",
        ),
        // Credit card patterns (various formats with spaces or dashes)
        (
            r"\b(?:\d{4}[- ]?){3}\d{4}\b",
            "regex",
            "deny",
            "Credit card number pattern (16 digits)",
        ),
        // Password mentions (case insensitive handled at enforcement time)
        (
            "password",
            "literal",
            "flag",
            "Message contains the word 'password'",
        ),
        // API key/secret patterns
        (
            r"\b(?:api[_-]?key|secret[_-]?key|access[_-]?token)\s*[:=]\s*\S+",
            "regex",
            "deny",
            "API key or secret assignment pattern",
        ),
    ]
}

fn row_to_queue_entry(row: &rusqlite::Row) -> rusqlite::Result<QueueEntry> {
    Ok(QueueEntry {
        id: row.get(0)?,
        api_key_id: row.get(1)?,
        channel: row.get(2)?,
        recipient_address: row.get(3)?,
        recipient_name: row.get(4)?,
        subject: row.get(5)?,
        body: row.get(6)?,
        priority: row.get(7)?,
        status: row.get(8)?,
        agent_context: row.get(9)?,
        created_at: parse_datetime(row.get::<_, String>(10)?),
        reviewed_at: row.get::<_, Option<String>>(11)?.map(parse_datetime),
        sent_at: row.get::<_, Option<String>>(12)?.map(parse_datetime),
        error_message: row.get(13)?,
    })
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowlist_crud() {
        let db = Database::open_memory().unwrap();

        // Create an API key first
        db.insert_api_key("key-1", "Test Agent", "hash123", "gw_abc")
            .unwrap();

        // Initially no allowlist
        assert!(!db.has_allowlist("key-1").unwrap());
        assert!(db.list_allowlist_entries("key-1").unwrap().is_empty());

        // Add allowlist entries
        assert!(db
            .insert_allowlist_entry("al-1", "key-1", "john@example.com")
            .unwrap());
        assert!(db
            .insert_allowlist_entry("al-2", "key-1", "*@acme.com")
            .unwrap());
        assert!(db
            .insert_allowlist_entry("al-3", "key-1", "+15551234567")
            .unwrap());

        // Now has allowlist
        assert!(db.has_allowlist("key-1").unwrap());

        // List entries
        let entries = db.list_allowlist_entries("key-1").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].recipient_pattern, "john@example.com");
        assert_eq!(entries[1].recipient_pattern, "*@acme.com");
        assert_eq!(entries[2].recipient_pattern, "+15551234567");

        // Idempotent insert (duplicate pattern)
        assert!(!db
            .insert_allowlist_entry("al-4", "key-1", "john@example.com")
            .unwrap());
        assert_eq!(db.list_allowlist_entries("key-1").unwrap().len(), 3);

        // Delete entry
        assert!(db
            .delete_allowlist_entry("key-1", "*@acme.com")
            .unwrap());
        assert_eq!(db.list_allowlist_entries("key-1").unwrap().len(), 2);

        // Delete non-existent entry
        assert!(!db
            .delete_allowlist_entry("key-1", "nonexistent@test.com")
            .unwrap());
    }

    #[test]
    fn test_api_key_crud() {
        let db = Database::open_memory().unwrap();

        // Insert a key
        db.insert_api_key("key-1", "Test Agent", "hash123", "gw_abc")
            .unwrap();

        // List keys
        let keys = db.list_api_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "Test Agent");
        assert_eq!(keys[0].key_prefix, "gw_abc");

        // Find by hash
        let found = db.find_api_key_by_hash("hash123").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "key-1");

        // Touch (update last_used_at)
        db.touch_api_key("key-1").unwrap();
        let key = db.find_api_key_by_hash("hash123").unwrap().unwrap();
        assert!(key.last_used_at.is_some());

        // Revoke
        assert!(db.revoke_api_key("key-1").unwrap());
        let key = db.find_api_key_by_hash("hash123").unwrap().unwrap();
        assert!(key.revoked_at.is_some());
    }

    #[test]
    fn test_queue_entry_crud() {
        let db = Database::open_memory().unwrap();

        // Need an API key first
        db.insert_api_key("key-1", "Test Agent", "hash123", "gw_abc")
            .unwrap();

        // Insert queue entry
        db.insert_queue_entry(
            "msg-1",
            "key-1",
            "email",
            "test@example.com",
            Some("Test User"),
            Some("Hello"),
            "This is a test email",
            "normal",
            Some(r#"{"reason": "test"}"#),
        )
        .unwrap();

        // Get entry
        let entry = db.get_queue_entry("msg-1").unwrap().unwrap();
        assert_eq!(entry.channel, "email");
        assert_eq!(entry.status, "pending");

        // List pending
        let pending = db.list_pending_queue().unwrap();
        assert_eq!(pending.len(), 1);

        // Count pending
        assert_eq!(db.count_pending_queue().unwrap(), 1);

        // Update status
        db.update_queue_status("msg-1", "approved").unwrap();
        let entry = db.get_queue_entry("msg-1").unwrap().unwrap();
        assert_eq!(entry.status, "approved");

        // Count pending (should be 0 now)
        assert_eq!(db.count_pending_queue().unwrap(), 0);

        // Add another entry and mark as sent
        db.insert_queue_entry(
            "msg-2",
            "key-1",
            "sms",
            "+15551234567",
            None,
            None,
            "Hello via SMS",
            "urgent",
            None,
        )
        .unwrap();

        db.mark_queue_sent("msg-2").unwrap();
        let entry = db.get_queue_entry("msg-2").unwrap().unwrap();
        assert_eq!(entry.status, "sent");
        assert!(entry.sent_at.is_some());
    }

    #[test]
    fn test_content_filter_seeding() {
        let db = Database::open_memory().unwrap();

        // Should have seeded default filters
        let filters = db.list_content_filters().unwrap();
        assert!(filters.len() >= 4, "Should have at least 4 default filters");

        // Check we have both regex and literal types
        let regex_count = filters.iter().filter(|f| f.pattern_type == "regex").count();
        let literal_count = filters.iter().filter(|f| f.pattern_type == "literal").count();
        assert!(regex_count > 0, "Should have regex filters");
        assert!(literal_count > 0, "Should have literal filters");

        // Check we have both deny and flag actions
        let deny_count = filters.iter().filter(|f| f.action == "deny").count();
        let flag_count = filters.iter().filter(|f| f.action == "flag").count();
        assert!(deny_count > 0, "Should have deny filters");
        assert!(flag_count > 0, "Should have flag filters");

        // All should be enabled by default
        assert!(filters.iter().all(|f| f.enabled), "All default filters should be enabled");
    }

    #[test]
    fn test_content_filter_crud() {
        let db = Database::open_memory().unwrap();

        // Insert a custom filter
        db.insert_content_filter(
            "filter-1",
            "confidential",
            "literal",
            "flag",
            Some("Flags confidential content"),
        )
        .unwrap();

        // Get filter by ID
        let filter = db.get_content_filter("filter-1").unwrap().unwrap();
        assert_eq!(filter.pattern, "confidential");
        assert_eq!(filter.pattern_type, "literal");
        assert_eq!(filter.action, "flag");
        assert!(filter.enabled);

        // Disable the filter
        assert!(db.set_content_filter_enabled("filter-1", false).unwrap());
        let filter = db.get_content_filter("filter-1").unwrap().unwrap();
        assert!(!filter.enabled);

        // Re-enable the filter
        assert!(db.set_content_filter_enabled("filter-1", true).unwrap());
        let filter = db.get_content_filter("filter-1").unwrap().unwrap();
        assert!(filter.enabled);

        // Delete the filter
        assert!(db.delete_content_filter("filter-1").unwrap());
        assert!(db.get_content_filter("filter-1").unwrap().is_none());
    }

    #[test]
    fn test_list_enabled_content_filters() {
        let db = Database::open_memory().unwrap();

        // Insert a disabled filter
        db.insert_content_filter(
            "filter-disabled",
            "test-pattern",
            "literal",
            "deny",
            None,
        )
        .unwrap();
        db.set_content_filter_enabled("filter-disabled", false).unwrap();

        // List enabled - should not include the disabled one
        let enabled = db.list_enabled_content_filters().unwrap();
        assert!(
            !enabled.iter().any(|f| f.id == "filter-disabled"),
            "Disabled filter should not appear in enabled list"
        );

        // List all - should include the disabled one
        let all = db.list_content_filters().unwrap();
        assert!(
            all.iter().any(|f| f.id == "filter-disabled"),
            "Disabled filter should appear in full list"
        );
    }

    #[test]
    fn test_content_filter_pattern_types_valid() {
        // Verify the default regex patterns are syntactically valid
        let filters = get_default_content_filters();

        for (pattern, pattern_type, _, description) in filters {
            if pattern_type == "regex" {
                let regex_result = regex::Regex::new(pattern);
                assert!(
                    regex_result.is_ok(),
                    "Invalid regex pattern for '{}': {:?}",
                    description,
                    regex_result.err()
                );
            }
        }
    }

    #[test]
    fn test_webhook_url_crud() {
        let db = Database::open_memory().unwrap();

        // Create an API key
        db.insert_api_key("key-1", "Test Agent", "hash123", "gw_abc")
            .unwrap();

        // Initially no webhook
        let webhook = db.get_api_key_webhook("key-1").unwrap();
        assert!(webhook.is_none());

        // Check via list_api_keys that webhook_url is None
        let keys = db.list_api_keys().unwrap();
        assert!(keys[0].webhook_url.is_none());

        // Set webhook URL
        assert!(db
            .set_api_key_webhook("key-1", Some("https://example.com/webhook"))
            .unwrap());

        // Verify it was set
        let webhook = db.get_api_key_webhook("key-1").unwrap();
        assert_eq!(webhook, Some("https://example.com/webhook".to_string()));

        // Check via list_api_keys
        let keys = db.list_api_keys().unwrap();
        assert_eq!(
            keys[0].webhook_url,
            Some("https://example.com/webhook".to_string())
        );

        // Update webhook URL
        assert!(db
            .set_api_key_webhook("key-1", Some("https://other.com/hook"))
            .unwrap());
        let webhook = db.get_api_key_webhook("key-1").unwrap();
        assert_eq!(webhook, Some("https://other.com/hook".to_string()));

        // Clear webhook URL
        assert!(db.set_api_key_webhook("key-1", None).unwrap());
        let webhook = db.get_api_key_webhook("key-1").unwrap();
        assert!(webhook.is_none());
    }
}
