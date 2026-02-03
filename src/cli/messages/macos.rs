//! macOS Messages database integration.
//!
//! Reads the most recent iMessage for a given contact by querying
//! ~/Library/Messages/chat.db

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone, Utc};
use imessage_database::util::streamtyped::parse as parse_typedstream;
use rusqlite::{Connection, OpenFlags};
use std::io::Write;
use std::path::PathBuf;

use crate::db::Database;
use super::super::display::format_message_date;
use super::super::ui::visible_lines;

/// Represents a message from the Messages app
#[derive(Debug, Clone)]
pub struct LastMessage {
    pub text: String,
    pub date: DateTime<Local>,
    pub is_from_me: bool,
    pub handle: String,
}

/// Get the path to the Messages database
fn messages_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    PathBuf::from(home).join("Library/Messages/chat.db")
}

/// Normalize a phone number to digits only, handling country code variations
fn normalize_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    // Handle US country code: if starts with 1 and is 11 digits, strip the 1
    if digits.len() == 11 && digits.starts_with('1') {
        digits[1..].to_string()
    } else {
        digits
    }
}

/// Check if two phone numbers match (after normalization)
fn phones_match(phone1: &str, phone2: &str) -> bool {
    let n1 = normalize_phone(phone1);
    let n2 = normalize_phone(phone2);

    if n1.is_empty() || n2.is_empty() {
        return false;
    }

    // Direct match
    if n1 == n2 {
        return true;
    }

    // Handle case where one has country code and other doesn't
    // e.g., "15551234567" should match "5551234567"
    if n1.len() > n2.len() {
        n1.ends_with(&n2)
    } else {
        n2.ends_with(&n1)
    }
}

/// Convert Apple's CoreData timestamp (nanoseconds since 2001-01-01) to DateTime
fn apple_timestamp_to_datetime(timestamp: i64) -> DateTime<Local> {
    // Apple uses nanoseconds since 2001-01-01 00:00:00 UTC
    // Unix epoch is 1970-01-01, so we need to add the difference
    const APPLE_EPOCH_OFFSET: i64 = 978307200; // seconds from 1970 to 2001

    // Timestamps after ~2017 are in nanoseconds, before that in seconds
    // We detect this by checking if the value is unreasonably large for seconds
    let seconds = if timestamp > 1_000_000_000_000_000 {
        // Nanoseconds - convert to seconds
        (timestamp / 1_000_000_000) + APPLE_EPOCH_OFFSET
    } else if timestamp > 1_000_000_000_000 {
        // Microseconds - some older formats
        (timestamp / 1_000_000) + APPLE_EPOCH_OFFSET
    } else {
        // Already seconds
        timestamp + APPLE_EPOCH_OFFSET
    };

    Utc.timestamp_opt(seconds, 0)
        .single()
        .map(|dt| dt.with_timezone(&Local))
        .unwrap_or_else(Local::now)
}

/// Open the Messages database, returning None if unavailable
fn open_messages_db() -> Result<Option<Connection>> {
    let db_path = messages_db_path();

    if !db_path.exists() {
        return Ok(None);
    }

    match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => Ok(Some(c)),
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("permission") || err_str.contains("unable to open") {
                return Ok(None);
            }
            Err(e).context("Failed to open Messages database")
        }
    }
}

/// Extract text from a message row, trying `text` column first, then `attributedBody`
fn extract_message_text(text: Option<String>, attributed_body: Option<Vec<u8>>) -> Option<String> {
    // Prefer the text column if it has readable content
    if let Some(ref t) = text {
        if !t.is_empty() {
            return Some(t.clone());
        }
    }

    // Fall back to parsing attributedBody blob with proper typedstream parser
    if let Some(blob) = attributed_body {
        if let Ok(parsed) = parse_typedstream(blob) {
            if !parsed.is_empty() {
                return Some(parsed);
            }
        }
    }

    None
}

/// Query the Messages database for the most recent message matching any of the given phone numbers
pub fn get_last_message_for_phones(phones: &[String]) -> Result<Option<LastMessage>> {
    get_last_message_for_handles(phones, &[])
}

/// Service type detected from chat history
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedService {
    IMessage,
    Sms,
    Unknown,
}

/// A recent message handle with metadata
#[derive(Debug, Clone)]
pub struct RecentHandle {
    pub handle: String,
    pub last_message_date: DateTime<Local>,
    pub service: DetectedService,
}

/// Detect the appropriate messaging service for a phone number based on chat history.
/// Returns the service type used in the most recent conversation with this number.
pub fn detect_service_for_phone(phone: &str) -> Result<DetectedService> {
    let conn = match open_messages_db()? {
        Some(c) => c,
        None => return Ok(DetectedService::Unknown),
    };

    // Build handle patterns for this phone
    let handle_patterns = phone_handle_patterns(phone);
    if handle_patterns.is_empty() {
        return Ok(DetectedService::Unknown);
    }

    // Query for the most recent chat with this handle to get the service type
    // Join chat -> chat_handle_join -> handle to find chats for this recipient
    let placeholders: Vec<String> = handle_patterns.iter().map(|_| "LOWER(h.id) = LOWER(?)".to_string()).collect();
    let query = format!(
        r#"SELECT c.service_name, c.last_read_message_timestamp
           FROM chat c
           INNER JOIN chat_handle_join chj ON c.ROWID = chj.chat_id
           INNER JOIN handle h ON chj.handle_id = h.ROWID
           WHERE ({})
           ORDER BY c.last_read_message_timestamp DESC
           LIMIT 1"#,
        placeholders.join(" OR ")
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(DetectedService::Unknown);
            }
            return Err(e).context("Failed to query chat service");
        }
    };

    let param_refs: Vec<&dyn rusqlite::ToSql> = handle_patterns
        .iter()
        .map(|p| p as &dyn rusqlite::ToSql)
        .collect();

    let result = stmt.query_row(param_refs.as_slice(), |row| {
        row.get::<_, String>(0)
    });

    match result {
        Ok(service_name) => {
            let service_lower = service_name.to_lowercase();
            if service_lower.contains("imessage") {
                Ok(DetectedService::IMessage)
            } else if service_lower.contains("sms") {
                Ok(DetectedService::Sms)
            } else {
                Ok(DetectedService::Unknown)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(DetectedService::Unknown),
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(DetectedService::Unknown);
            }
            Err(e).context("Failed to detect service")
        }
    }
}

/// Get all unique handles with recent messages in the last N days
///
/// Returns handles sorted by most recent message date (descending)
pub fn get_recent_message_handles(days: u32) -> Result<Vec<RecentHandle>> {
    let conn = match open_messages_db()? {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    // Calculate the cutoff timestamp (days ago in Apple's nanosecond format)
    let now = Local::now();
    let cutoff = now - chrono::Duration::days(days as i64);
    let cutoff_unix = cutoff.timestamp();
    const APPLE_EPOCH_OFFSET: i64 = 978307200;
    let cutoff_apple_seconds = cutoff_unix - APPLE_EPOCH_OFFSET;
    // Modern messages use nanoseconds
    let cutoff_apple_ns = cutoff_apple_seconds * 1_000_000_000;

    // Query for unique handles with their most recent message and service type
    // Join through chat_message_join -> chat -> chat_handle_join -> handle for service info
    let query = r#"
        SELECT
            h.id as handle,
            MAX(m.date) as last_date,
            (SELECT c.service_name
             FROM chat c
             INNER JOIN chat_handle_join chj ON c.ROWID = chj.chat_id
             WHERE chj.handle_id = h.ROWID
             ORDER BY c.last_read_message_timestamp DESC
             LIMIT 1) as service_name
        FROM message m
        INNER JOIN handle h ON m.handle_id = h.ROWID
        WHERE m.date >= ?
        GROUP BY h.id
        ORDER BY last_date DESC
    "#;

    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(vec![]);
            }
            return Err(e).context("Failed to query recent handles");
        }
    };

    let rows = stmt.query_map([cutoff_apple_ns], |row| {
        let handle: String = row.get(0)?;
        let date: i64 = row.get(1)?;
        let service_name: Option<String> = row.get(2)?;

        let service = match service_name {
            Some(s) => {
                let s_lower = s.to_lowercase();
                if s_lower.contains("imessage") {
                    DetectedService::IMessage
                } else if s_lower.contains("sms") {
                    DetectedService::Sms
                } else {
                    DetectedService::Unknown
                }
            }
            None => DetectedService::Unknown,
        };

        Ok((handle, date, service))
    })?;

    let mut results = Vec::new();
    for row_result in rows {
        match row_result {
            Ok((handle, date, service)) => {
                results.push(RecentHandle {
                    handle,
                    last_message_date: apple_timestamp_to_datetime(date),
                    service,
                });
            }
            Err(_) => continue,
        }
    }

    Ok(results)
}

/// Make phones_match public for use by other modules
pub fn phones_match_public(phone1: &str, phone2: &str) -> bool {
    phones_match(phone1, phone2)
}

/// Generate possible handle formats for a phone number
fn phone_handle_patterns(phone: &str) -> Vec<String> {
    let digits = normalize_phone(phone);
    if digits.is_empty() {
        return vec![];
    }

    // Generate common formats: with/without +1 prefix
    let mut patterns = vec![
        digits.clone(),
        format!("+1{}", digits),
        format!("+{}", digits),
    ];

    // If it's 10 digits, also try with 1 prefix (no +)
    if digits.len() == 10 {
        patterns.push(format!("1{}", digits));
    }

    patterns
}

/// Query the Messages database for the most recent message matching phones OR emails
pub fn get_last_message_for_handles(phones: &[String], emails: &[String]) -> Result<Option<LastMessage>> {
    if phones.is_empty() && emails.is_empty() {
        return Ok(None);
    }

    let conn = match open_messages_db()? {
        Some(c) => c,
        None => return Ok(None),
    };

    // Build targeted handle patterns
    let mut handle_patterns: Vec<String> = Vec::new();
    for phone in phones {
        handle_patterns.extend(phone_handle_patterns(phone));
    }
    for email in emails {
        handle_patterns.push(email.to_lowercase());
    }

    if handle_patterns.is_empty() {
        return Ok(None);
    }

    // Query handles matching our specific patterns
    let placeholders: Vec<String> = handle_patterns.iter().map(|_| "LOWER(id) = LOWER(?)".to_string()).collect();
    let handle_query = format!(
        "SELECT ROWID FROM handle WHERE {}",
        placeholders.join(" OR ")
    );

    let mut handle_stmt = match conn.prepare(&handle_query) {
        Ok(s) => s,
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(None);
            }
            return Err(e).context("Failed to query handles");
        }
    };

    let param_refs: Vec<&dyn rusqlite::ToSql> = handle_patterns
        .iter()
        .map(|p| p as &dyn rusqlite::ToSql)
        .collect();

    let handle_rows = handle_stmt.query_map(param_refs.as_slice(), |row| {
        row.get::<_, i64>(0)
    })?;

    let matching_rowids: Vec<i64> = handle_rows.filter_map(|r| r.ok()).collect();

    if matching_rowids.is_empty() {
        return Ok(None);
    }

    // Query the most recent message for matching handles
    let msg_placeholders: Vec<String> = matching_rowids.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        r#"SELECT m.text, m.attributedBody, m.date, m.is_from_me, h.id
           FROM message m
           INNER JOIN handle h ON m.handle_id = h.ROWID
           WHERE m.handle_id IN ({})
           ORDER BY m.date DESC
           LIMIT 10"#,
        msg_placeholders.join(", ")
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            return Err(e).context("Failed to prepare message query");
        }
    };

    let rowid_refs: Vec<&dyn rusqlite::ToSql> = matching_rowids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();

    let rows = stmt.query_map(rowid_refs.as_slice(), |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<Vec<u8>>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i32>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    for row_result in rows {
        let (text, attributed_body, date, is_from_me, handle) = match row_result {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Some(msg_text) = extract_message_text(text, attributed_body) {
            return Ok(Some(LastMessage {
                text: msg_text,
                date: apple_timestamp_to_datetime(date),
                is_from_me: is_from_me != 0,
                handle,
            }));
        }
    }

    Ok(None)
}

/// Query the Messages database for multiple recent messages matching any of the given phone numbers
pub fn get_messages_for_phones(phones: &[String], limit: u32) -> Result<Vec<LastMessage>> {
    get_messages_for_handles(phones, &[], limit)
}

/// Query the Messages database for messages matching phone numbers OR email addresses
pub fn get_messages_for_handles(phones: &[String], emails: &[String], limit: u32) -> Result<Vec<LastMessage>> {
    if phones.is_empty() && emails.is_empty() {
        return Ok(vec![]);
    }

    let conn = match open_messages_db()? {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    // Build targeted handle patterns for this contact only
    let mut handle_patterns: Vec<String> = Vec::new();
    for phone in phones {
        handle_patterns.extend(phone_handle_patterns(phone));
    }
    for email in emails {
        handle_patterns.push(email.to_lowercase());
    }

    if handle_patterns.is_empty() {
        return Ok(vec![]);
    }

    // Query handles matching our specific patterns
    let placeholders: Vec<String> = handle_patterns.iter().map(|_| "LOWER(id) = LOWER(?)".to_string()).collect();
    let handle_query = format!(
        "SELECT ROWID FROM handle WHERE {}",
        placeholders.join(" OR ")
    );

    let mut handle_stmt = match conn.prepare(&handle_query) {
        Ok(s) => s,
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(vec![]);
            }
            return Err(e).context("Failed to query handles");
        }
    };

    let param_refs: Vec<&dyn rusqlite::ToSql> = handle_patterns
        .iter()
        .map(|p| p as &dyn rusqlite::ToSql)
        .collect();

    let handle_rows = handle_stmt.query_map(param_refs.as_slice(), |row| {
        row.get::<_, i64>(0)
    })?;

    let matching_rowids: Vec<i64> = handle_rows.filter_map(|r| r.ok()).collect();

    if matching_rowids.is_empty() {
        return Ok(vec![]);
    }

    // Query messages for matching handles only
    let msg_placeholders: Vec<String> = matching_rowids.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        r#"SELECT m.text, m.attributedBody, m.date, m.is_from_me, h.id
           FROM message m
           INNER JOIN handle h ON m.handle_id = h.ROWID
           WHERE m.handle_id IN ({})
           ORDER BY m.date DESC
           LIMIT ?"#,
        msg_placeholders.join(", ")
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            return Err(e).context("Failed to prepare message query");
        }
    };

    // Build params: handle ROWIDs + limit
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = matching_rowids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(limit));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<Vec<u8>>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i32>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut messages = Vec::new();
    for row_result in rows {
        let (text, attributed_body, date, is_from_me, handle) = match row_result {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Some(msg_text) = extract_message_text(text, attributed_body) {
            messages.push(LastMessage {
                text: msg_text,
                date: apple_timestamp_to_datetime(date),
                is_from_me: is_from_me != 0,
                handle,
            });
        }
    }

    Ok(messages)
}

/// Search messages using SQL LIKE for full database coverage
///
/// Queries the full Messages database with SQL filtering instead of loading
/// into memory. Supports date filtering via `since_date` (YYYY-MM-DD format).
pub fn search_messages(terms: &[&str], limit: u32, since_date: Option<&str>) -> Result<Vec<LastMessage>> {
    if terms.is_empty() {
        return Ok(vec![]);
    }

    let conn = match open_messages_db()? {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    // Build SQL WHERE clause:
    // - Messages where text matches ALL terms (SQL-optimized path)
    // - OR messages where text is NULL (need post-processing for attributedBody)
    let text_likes: Vec<String> = terms
        .iter()
        .map(|_| "m.text LIKE ?".to_string())
        .collect();
    let text_filter = format!("(({}) OR m.text IS NULL)", text_likes.join(" AND "));

    let mut where_clauses = vec![text_filter];

    // Add date filter if provided
    let since_timestamp = since_date.and_then(parse_date_to_apple_timestamp);
    if since_timestamp.is_some() {
        where_clauses.push("m.date >= ?".to_string());
    }

    let query = format!(
        r#"SELECT m.text, m.attributedBody, m.date, m.is_from_me, h.id
           FROM message m
           INNER JOIN handle h ON m.handle_id = h.ROWID
           WHERE {}
           ORDER BY m.date DESC
           LIMIT ?"#,
        where_clauses.join(" AND ")
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("locked") || err_str.contains("encrypted") {
                return Ok(vec![]);
            }
            return Err(e).context("Failed to query Messages database");
        }
    };

    // Build params: %term% patterns + optional since_timestamp + limit
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = terms
        .iter()
        .map(|t| Box::new(format!("%{}%", t)) as Box<dyn rusqlite::ToSql>)
        .collect();

    if let Some(ts) = since_timestamp {
        params.push(Box::new(ts));
    }
    params.push(Box::new(limit));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<Vec<u8>>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i32>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let lower_terms: Vec<String> = terms.iter().map(|t| t.to_lowercase()).collect();
    let mut results = Vec::new();

    for row_result in rows {
        let (text, attributed_body, date, is_from_me, handle) = match row_result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let msg_text = match extract_message_text(text, attributed_body) {
            Some(t) => t,
            None => continue,
        };

        // Double-check term matching (SQL LIKE is case-insensitive on text column,
        // but we may have extracted from attributedBody)
        let text_lower = msg_text.to_lowercase();
        let all_match = lower_terms.iter().all(|term| text_lower.contains(term.as_str()));

        if all_match {
            results.push(LastMessage {
                text: msg_text,
                date: apple_timestamp_to_datetime(date),
                is_from_me: is_from_me != 0,
                handle,
            });
        }
    }

    Ok(results)
}

/// Parse a YYYY-MM-DD date string to Apple timestamp (nanoseconds since 2001-01-01)
fn parse_date_to_apple_timestamp(date_str: &str) -> Option<i64> {
    use chrono::NaiveDate;

    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    let datetime = date.and_hms_opt(0, 0, 0)?;

    // Convert to Unix timestamp then to Apple timestamp
    let unix_ts = datetime.and_utc().timestamp();
    const APPLE_EPOCH_OFFSET: i64 = 978307200; // seconds from 1970 to 2001
    let apple_seconds = unix_ts - APPLE_EPOCH_OFFSET;

    // Return in nanoseconds (modern Messages format)
    Some(apple_seconds * 1_000_000_000)
}

/// A group of messages from a single handle, with resolved contact info
struct MessageGroup {
    handle: String,
    contact_name: String,
    location: Option<String>,
    messages: Vec<LastMessage>,
}

/// Extract a snippet of text centered around the first occurrence of a search term
fn snippet_around_match(text: &str, search_terms: &[String], max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let trimmed = first_line.trim();
    let char_count = trimmed.chars().count();

    // If text fits, return as-is
    if char_count <= max_len {
        return trimmed.to_string();
    }

    let text_lower = trimmed.to_lowercase();

    // Find the first matching term and its position
    let mut best_pos: Option<usize> = None;
    for term in search_terms {
        if let Some(byte_pos) = text_lower.find(term.as_str()) {
            // Convert byte position to character position
            let char_pos = text_lower[..byte_pos].chars().count();
            if best_pos.is_none() || char_pos < best_pos.unwrap() {
                best_pos = Some(char_pos);
            }
        }
    }

    let match_pos = match best_pos {
        Some(pos) => pos,
        None => return truncate_to_width(text, max_len), // Fallback to start
    };

    // Calculate window around match (leave room for ellipsis on both sides)
    let content_len = max_len - 6; // "..." on each side
    let half_window = content_len / 2;

    let (start, end, prefix, suffix) = if match_pos <= half_window {
        // Match is near the start - show from beginning
        let end = std::cmp::min(max_len - 3, char_count);
        (0, end, "", "...")
    } else if match_pos + half_window >= char_count {
        // Match is near the end - show the end
        let start = char_count.saturating_sub(max_len - 3);
        (start, char_count, "...", "")
    } else {
        // Match is in the middle - center around it
        let start = match_pos.saturating_sub(half_window);
        let end = std::cmp::min(start + content_len, char_count);
        (start, end, "...", "...")
    };

    let snippet: String = trimmed.chars().skip(start).take(end - start).collect();
    format!("{}{}{}", prefix, snippet.trim(), suffix)
}

/// RAII guard that ensures raw mode is disabled on drop
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Run the messages search command with interactive review-style navigation
pub fn run_messages(db: &Database, query: &str, since: Option<&str>) -> Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEvent};
    use crate::cli::ui::clear_screen;

    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        println!("Usage: contactcmd messages \"search terms\" [--since YYYY-MM-DD]");
        return Ok(());
    }

    // Keep lowercase search terms for snippet centering
    let search_terms: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

    // Search full history with high limit (10000)
    let results = search_messages(&words, 10000, since)?;

    if results.is_empty() {
        println!("No messages found matching \"{}\".", query);
        return Ok(());
    }

    let total_matches = results.len();

    // Group results by handle, preserving insertion order
    let mut handle_order: Vec<String> = Vec::new();
    let mut by_handle: std::collections::HashMap<String, Vec<LastMessage>> =
        std::collections::HashMap::new();
    for msg in results {
        if !by_handle.contains_key(&msg.handle) {
            handle_order.push(msg.handle.clone());
        }
        by_handle.entry(msg.handle.clone()).or_default().push(msg);
    }

    // Resolve each handle to contact name + location
    let all_persons = db.list_persons(u32::MAX, 0)?;
    let mut groups: Vec<MessageGroup> = Vec::new();

    for handle in handle_order {
        let messages = by_handle.remove(&handle).unwrap_or_default();
        let (contact_name, location) = resolve_handle_info(db, &handle, &all_persons);
        groups.push(MessageGroup {
            handle,
            contact_name,
            location,
            messages,
        });
    }

    // Interactive review loop
    let mut index: usize = 0;
    let mut scroll: usize = 0;
    let mut selected: usize = 0; // Selected message within the visible window

    loop {
        let num_visible = visible_lines(); // Recalculate for resize support
        let group = &groups[index];
        let total_msgs = group.messages.len();

        // Ensure selected stays within bounds
        let max_selected = std::cmp::min(num_visible, total_msgs.saturating_sub(scroll));
        if selected >= max_selected && max_selected > 0 {
            selected = max_selected - 1;
        }

        clear_screen()?;

        // Header with total match count
        println!("Messages: {} ({} matches across {} contacts)\n",
            group.contact_name,
            total_matches,
            groups.len()
        );

        // Subheader: location and handle
        let loc = group.location.as_deref().unwrap_or("");
        if !loc.is_empty() {
            println!("  {}", loc);
        }
        println!("  {}\n", group.handle);

        // Messages with scroll offset
        let end = std::cmp::min(scroll + num_visible, total_msgs);
        for (i, msg) in group.messages[scroll..end].iter().enumerate() {
            let direction = if msg.is_from_me { ">" } else { "<" };
            let date_str = format_message_date(&msg.date);
            let text = snippet_around_match(&msg.text, &search_terms, 50);

            // Mark selected message with indicator
            let marker = if i == selected { ">" } else { " " };
            println!("{} {} {} \"{}\"", marker, direction, date_str, text);
        }

        // Pad to keep footer in a stable position
        let displayed = end - scroll;
        for _ in displayed..num_visible {
            println!();
        }

        // Footer
        println!();
        let range_str = if total_msgs > num_visible {
            format!("  {}-{} of {}", scroll + 1, end, total_msgs)
        } else {
            String::new()
        };
        println!(
            "{} of {}  {} message(s){}",
            index + 1,
            groups.len(),
            total_msgs,
            range_str
        );
        print!("[←/→] contact [↑/↓] select [enter] expand [q]uit: ");
        std::io::stdout().flush()?;

        // Read single key with RAII guard for raw mode
        let action = {
            let _guard = RawModeGuard::new()?;
            match event::read()? {
                Event::Key(KeyEvent { code, .. }) => code,
                _ => continue,
            }
        };

        match action {
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                if index + 1 < groups.len() {
                    index += 1;
                    scroll = 0;
                    selected = 0;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if index > 0 {
                    index -= 1;
                    scroll = 0;
                    selected = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_idx = std::cmp::min(num_visible, total_msgs.saturating_sub(scroll));
                if selected + 1 < max_idx {
                    // Move selection down within visible window
                    selected += 1;
                } else if scroll + num_visible < total_msgs {
                    // Scroll down and keep selection at bottom
                    scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if selected > 0 {
                    // Move selection up within visible window
                    selected -= 1;
                } else {
                    // Scroll up and keep selection at top
                    scroll = scroll.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                // Show expanded message
                let msg_index = scroll + selected;
                if msg_index < total_msgs {
                    show_expanded_message(
                        &group.messages[msg_index],
                        &group.contact_name,
                    )?;
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                break;
            }
            _ => {}
        }
    }

    clear_screen()?;
    Ok(())
}

/// Display the full message content in an expanded view
fn show_expanded_message(
    msg: &LastMessage,
    contact_name: &str,
) -> Result<()> {
    use crossterm::event;
    use crate::cli::ui::clear_screen;

    clear_screen()?;

    // Header
    let direction = if msg.is_from_me { ">" } else { "<" };
    println!("{} {}\n", direction, contact_name);

    // Metadata
    let date_str = format_message_date(&msg.date);
    println!("  {}", date_str);
    println!("  {}\n", msg.handle);

    // Full message text with word wrapping
    let wrapped = wrap_text(&msg.text, 76);
    for line in &wrapped {
        println!("  {}", line);
    }

    println!();
    print!("[enter] or [q]uit: ");
    std::io::stdout().flush()?;

    // Wait for key with RAII guard
    let _guard = RawModeGuard::new()?;
    let _ = event::read();

    Ok(())
}

/// Wrap text to specified width (character-aware for Unicode safety)
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0usize;

        for word in paragraph.split_whitespace() {
            let word_width = word.chars().count();

            if current_line.is_empty() {
                // Word longer than width - take it anyway (antifragile: don't lose data)
                current_line = word.to_string();
                current_width = word_width;
            } else if current_width + 1 + word_width <= width {
                current_line.push(' ');
                current_line.push_str(word);
                current_width += 1 + word_width;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    lines
}

/// Resolve a Messages handle to contact name + location (city, state)
fn resolve_handle_info(
    db: &Database,
    handle: &str,
    all_persons: &[crate::models::Person],
) -> (String, Option<String>) {
    for person in all_persons {
        let phones = match db.get_phones_for_person(person.id) {
            Ok(p) => p,
            Err(_) => continue,
        };
        for phone in &phones {
            if phones_match(handle, &phone.phone_number) {
                let name = person
                    .display_name
                    .clone()
                    .unwrap_or_else(|| handle.to_string());
                let location = person_location(db, person.id);
                return (name, location);
            }
        }

        let emails = match db.get_emails_for_person(person.id) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for email in &emails {
            if email.email_address.eq_ignore_ascii_case(handle) {
                let name = person
                    .display_name
                    .clone()
                    .unwrap_or_else(|| handle.to_string());
                let location = person_location(db, person.id);
                return (name, location);
            }
        }
    }

    (handle.to_string(), None)
}

/// Get city/state for a person
fn person_location(db: &Database, person_id: uuid::Uuid) -> Option<String> {
    let detail = db.get_contact_detail(person_id).ok()??;
    let addr = detail.addresses.first()?;
    match (&addr.city, &addr.state) {
        (Some(c), Some(s)) => Some(format!("{}, {}", c, s)),
        (Some(c), None) => Some(c.clone()),
        (None, Some(s)) => Some(s.clone()),
        (None, None) => None,
    }
}

/// Truncate text to terminal width
fn truncate_to_width(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let trimmed = first_line.trim();
    if trimmed.chars().count() <= max_len {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_len - 3).collect();
        format!("{}...", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_normalize_phone() {
        assert_eq!(normalize_phone("+1 (555) 123-4567"), "5551234567");
        assert_eq!(normalize_phone("555-123-4567"), "5551234567");
        assert_eq!(normalize_phone("15551234567"), "5551234567");
        assert_eq!(normalize_phone("5551234567"), "5551234567");
    }

    #[test]
    fn test_phones_match() {
        assert!(phones_match("+1 (555) 123-4567", "5551234567"));
        assert!(phones_match("15551234567", "555-123-4567"));
        assert!(phones_match("5551234567", "5551234567"));
        assert!(!phones_match("5551234567", "5551234568"));
        assert!(!phones_match("", "5551234567"));
    }

    #[test]
    fn test_apple_timestamp_conversion() {
        // Test a known timestamp (roughly 2024)
        let ts = 727_000_000_000_000_000i64; // nanoseconds
        let dt = apple_timestamp_to_datetime(ts);
        // Should be a reasonable date (after 2020)
        assert!(dt.year() >= 2020);
    }

    #[test]
    fn test_empty_phones() {
        let result = get_last_message_for_phones(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_extract_message_text_prefers_text_column() {
        // When text column has content, use it directly
        let result = extract_message_text(Some("Hello world".to_string()), None);
        assert_eq!(result, Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_message_text_empty_text_falls_back() {
        // Empty text column should fall back to attributedBody (returns None if blob also empty/invalid)
        let result = extract_message_text(Some("".to_string()), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_message_text_none_inputs() {
        // Both None should return None
        let result = extract_message_text(None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_snippet_around_match_short_text() {
        // Text fits within max_len, return as-is
        let terms = vec!["hello".to_string()];
        let result = snippet_around_match("Hello world", &terms, 52);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_snippet_around_match_term_at_start() {
        // Search term near start - should show from beginning with ellipsis at end
        let terms = vec!["hello".to_string()];
        let text = "Hello world, this is a very long message that exceeds the maximum length allowed";
        let result = snippet_around_match(text, &terms, 40);
        assert!(result.starts_with("Hello"));
        assert!(result.ends_with("..."));
        assert!(result.len() <= 40);
    }

    #[test]
    fn test_snippet_around_match_term_in_middle() {
        // Search term in middle - should show snippet centered around match
        let terms = vec!["sexy".to_string()];
        let text = "I'm 28 years old and just graduated from university and I think this is super sexy and I killed it with my presentation today";
        let result = snippet_around_match(text, &terms, 52);
        // Should contain the search term
        assert!(result.to_lowercase().contains("sexy"));
        // Should have ellipsis on both ends since match is in middle
        assert!(result.starts_with("...") || result.ends_with("..."));
    }

    #[test]
    fn test_snippet_around_match_term_at_end() {
        // Search term near end - should show end with ellipsis at start
        let terms = vec!["done".to_string()];
        let text = "This is a very long message that keeps going and going until finally we are done";
        let result = snippet_around_match(text, &terms, 40);
        assert!(result.to_lowercase().contains("done"));
        assert!(result.starts_with("..."));
    }

    #[test]
    fn test_snippet_around_match_no_match_fallback() {
        // No match found - should fall back to truncate_to_width behavior
        let terms = vec!["xyz".to_string()];
        let text = "This is a message that doesn't contain the search term at all";
        let result = snippet_around_match(text, &terms, 30);
        // Should truncate from start
        assert!(result.starts_with("This"));
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_wrap_text_unicode_width() {
        // Japanese characters - each counts as 1 char for our purposes
        let text = "これは日本語のテストです";
        let wrapped = wrap_text(text, 5);
        // Should not panic, should produce some output
        assert!(!wrapped.is_empty());
    }

    #[test]
    fn test_wrap_text_long_word() {
        // Antifragile: very long word shouldn't cause issues
        let text = "pneumonoultramicroscopicsilicovolcanoconiosis is a word";
        let wrapped = wrap_text(text, 10);
        // Long word should be kept intact (don't lose data)
        assert!(wrapped.iter().any(|line| line.contains("pneumono")));
    }
}
