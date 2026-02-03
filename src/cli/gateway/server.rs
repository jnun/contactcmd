//! HTTP server for the communication gateway.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::execute;
use super::filter::{ContentFilterMatcher, FilterResult};
use super::keys;
use super::webhook;
use super::types::{
    ActionStatusResponse, AllowlistErrorResponse, ConsentDeniedErrorResponse,
    ContentBlockedErrorResponse, GatewayApiResponse, GatewayChannel, HealthResponse,
    QueueEntryResponse, QueueListResponse, QueueStatus, RateLimitErrorResponse, SendRequest,
    SendResponse,
};
use crate::db::Database;

/// HTTP server for the communication gateway.
pub struct GatewayServer {
    port: u16,
    db_path: PathBuf,
    start_time: Instant,
    content_filter: ContentFilterMatcher,
}

impl GatewayServer {
    /// Create a new gateway server.
    pub fn new(port: u16, db: &Database) -> Result<Self> {
        // Get the database path for opening new connections in request handlers
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
        let db_path = config_dir.join("contactcmd").join("contacts.db");

        // Verify DB is accessible
        let _ = db.count_pending_queue()?;

        // Initialize and load content filters (compiled once for performance)
        let content_filter = ContentFilterMatcher::new();
        let filter_count = content_filter.reload(db)?;
        if filter_count > 0 {
            println!("Loaded {} content filter(s)", filter_count);
        }

        Ok(Self {
            port,
            db_path,
            start_time: Instant::now(),
            content_filter,
        })
    }

    /// Start the server (blocking).
    pub fn start(&self, shutdown: Arc<AtomicBool>) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        listener.set_nonblocking(true)?;

        println!("Gateway server listening on 0.0.0.0:{}", self.port);

        while !shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    if let Err(e) = self.handle_connection(stream, peer_addr) {
                        eprintln!("Request error: {}", e);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;

        let mut reader = BufReader::new(stream.try_clone()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return self.send_response(&mut stream, 400, "Bad Request");
        }

        let method = parts[0];
        let path = parts[1];

        // Parse headers
        let mut headers = HashMap::new();
        let mut content_length = 0usize;

        loop {
            let mut header_line = String::new();
            reader.read_line(&mut header_line)?;
            let header_line = header_line.trim();
            if header_line.is_empty() {
                break;
            }
            if let Some((key, value)) = header_line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                if key == "content-length" {
                    content_length = value.parse().unwrap_or(0);
                }
                headers.insert(key, value);
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            std::io::Read::read_exact(&mut reader, &mut body)?;
        }

        // Check if request is from localhost
        let is_local = peer_addr.ip().is_loopback();

        // Route request
        match (method, path) {
            // Public endpoints (require API key)
            ("GET", "/gateway/health") => self.handle_health(&mut stream),
            ("POST", "/gateway/send") => self.handle_send(&mut stream, &headers, &body),
            ("GET", p) if p.starts_with("/gateway/actions/") => {
                let id = p.strip_prefix("/gateway/actions/").unwrap_or("");
                self.handle_action_status(&mut stream, &headers, id)
            }

            // Local-only endpoints
            ("GET", "/gateway/queue") if is_local => self.handle_list_queue(&mut stream),
            ("POST", p) if is_local && p.starts_with("/gateway/queue/") && p.ends_with("/approve") => {
                let id = p
                    .strip_prefix("/gateway/queue/")
                    .and_then(|s| s.strip_suffix("/approve"))
                    .unwrap_or("");
                self.handle_approve(&mut stream, id)
            }
            ("POST", p) if is_local && p.starts_with("/gateway/queue/") && p.ends_with("/deny") => {
                let id = p
                    .strip_prefix("/gateway/queue/")
                    .and_then(|s| s.strip_suffix("/deny"))
                    .unwrap_or("");
                self.handle_deny(&mut stream, id)
            }

            // Local-only but accessed from non-local
            ("GET", "/gateway/queue") => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err("This endpoint is only accessible from localhost");
                self.send_json_response(&mut stream, 403, &response)
            }
            ("POST", p) if p.contains("/gateway/queue/") => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err("This endpoint is only accessible from localhost");
                self.send_json_response(&mut stream, 403, &response)
            }

            _ => self.send_response(&mut stream, 404, "Not Found"),
        }
    }

    /// Health check endpoint.
    fn handle_health(&self, stream: &mut TcpStream) -> Result<()> {
        let db = Database::open_at(self.db_path.clone())?;
        let pending_count = db.count_pending_queue().unwrap_or(0);

        let health = HealthResponse {
            status: "ok".to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            pending_count,
            version: "1.0".to_string(),
        };

        let response = GatewayApiResponse::ok(health);
        self.send_json_response(stream, 200, &response)
    }

    /// Queue a message for approval.
    fn handle_send(
        &self,
        stream: &mut TcpStream,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<()> {
        // Authenticate
        let api_key = match self.authenticate(headers) {
            Ok(key) => key,
            Err(e) => {
                let response: GatewayApiResponse<()> = GatewayApiResponse::err(e.to_string());
                return self.send_json_response(stream, 401, &response);
            }
        };

        // Open database early for rate limit check
        let db = Database::open_at(self.db_path.clone())?;

        // Check rate limits
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let one_day_ago = now - chrono::Duration::days(1);

        let hourly_count = db.count_queue_since(&api_key.id, one_hour_ago)?;
        if hourly_count >= api_key.rate_limit_per_hour as i64 {
            let response = RateLimitErrorResponse {
                error: "rate_limit_exceeded".to_string(),
                retry_after_seconds: 3600,
                limit_type: "hourly".to_string(),
                current_count: hourly_count,
                limit: api_key.rate_limit_per_hour,
            };
            return self.send_json_response(stream, 429, &response);
        }

        let daily_count = db.count_queue_since(&api_key.id, one_day_ago)?;
        if daily_count >= api_key.rate_limit_per_day as i64 {
            let response = RateLimitErrorResponse {
                error: "rate_limit_exceeded".to_string(),
                retry_after_seconds: 86400,
                limit_type: "daily".to_string(),
                current_count: daily_count,
                limit: api_key.rate_limit_per_day,
            };
            return self.send_json_response(stream, 429, &response);
        }

        // Parse request
        let req: SendRequest = match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(e) => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err(format!("Invalid request: {}", e));
                return self.send_json_response(stream, 400, &response);
            }
        };

        // Validate
        if req.body.trim().is_empty() {
            let response: GatewayApiResponse<()> =
                GatewayApiResponse::err("Message body cannot be empty");
            return self.send_json_response(stream, 400, &response);
        }

        if req.recipient_address.trim().is_empty() {
            let response: GatewayApiResponse<()> =
                GatewayApiResponse::err("Recipient address is required");
            return self.send_json_response(stream, 400, &response);
        }

        // Email requires subject
        if matches!(req.channel, GatewayChannel::Email) && req.subject.is_none() {
            let response: GatewayApiResponse<()> =
                GatewayApiResponse::err("Email requires a subject");
            return self.send_json_response(stream, 400, &response);
        }

        // Check recipient allowlist
        let allowlist = db.list_allowlist_entries(&api_key.id)?;
        if !allowlist.is_empty() {
            let patterns: Vec<String> = allowlist.iter().map(|e| e.recipient_pattern.clone()).collect();
            if !recipient_matches_allowlist(&req.recipient_address, &patterns) {
                let response = AllowlistErrorResponse {
                    error: "recipient_not_allowed".to_string(),
                    allowed_patterns: patterns,
                };
                return self.send_json_response(stream, 403, &response);
            }
        }

        // Check contact AI consent flag
        // Look up recipient by email or phone to check if they've opted out
        let contact = if req.recipient_address.contains('@') {
            db.get_person_by_email(&req.recipient_address)?
        } else {
            db.get_person_by_phone(&req.recipient_address)?
        };

        if let Some(person) = contact {
            if !person.ai_contact_allowed {
                let response = ConsentDeniedErrorResponse {
                    error: "contact_consent_denied".to_string(),
                    recipient: req.recipient_address.clone(),
                };
                return self.send_json_response(stream, 403, &response);
            }
        }

        // Check content filters (subject + body for email, body only for SMS/iMessage)
        let filter_result = if matches!(req.channel, GatewayChannel::Email) {
            self.content_filter.check_email(req.subject.as_deref(), &req.body)
        } else {
            self.content_filter.check_message(&req.body)
        };

        // Handle deny filter match - reject immediately
        if let FilterResult::Denied {
            filter_name,
            description,
        } = filter_result
        {
            let response = ContentBlockedErrorResponse {
                error: "content_blocked".to_string(),
                filter: filter_name,
                description,
            };
            return self.send_json_response(stream, 400, &response);
        }

        // Determine initial status (flagged if filter matched, pending otherwise)
        let initial_status = if matches!(filter_result, FilterResult::Flagged { .. }) {
            "flagged"
        } else {
            "pending"
        };

        // Insert into queue (db already opened for rate limit check)
        let id = uuid::Uuid::new_v4().to_string();
        let context_json = req.context.map(|c| serde_json::to_string(&c).ok()).flatten();

        db.insert_queue_entry(
            &id,
            &api_key.id,
            &req.channel.to_string(),
            &req.recipient_address,
            req.recipient_name.as_deref(),
            req.subject.as_deref(),
            &req.body,
            &req.priority.to_string(),
            context_json.as_deref(),
        )?;

        // If flagged, update status from pending to flagged
        if initial_status == "flagged" {
            db.update_queue_status(&id, "flagged")?;
        }

        // Update key last_used
        db.touch_api_key(&api_key.id)?;

        let response_status = if initial_status == "flagged" {
            QueueStatus::Flagged
        } else {
            QueueStatus::Pending
        };

        let response = GatewayApiResponse::ok(SendResponse {
            action_id: id,
            status: response_status,
        });
        self.send_json_response(stream, 200, &response)
    }

    /// Get action status.
    fn handle_action_status(
        &self,
        stream: &mut TcpStream,
        headers: &HashMap<String, String>,
        id: &str,
    ) -> Result<()> {
        // Authenticate
        if let Err(e) = self.authenticate(headers) {
            let response: GatewayApiResponse<()> = GatewayApiResponse::err(e.to_string());
            return self.send_json_response(stream, 401, &response);
        }

        let db = Database::open_at(self.db_path.clone())?;

        match db.get_queue_entry(id)? {
            Some(entry) => {
                let status: QueueStatus = entry.status.parse().unwrap_or(QueueStatus::Pending);
                let response = GatewayApiResponse::ok(ActionStatusResponse {
                    action_id: entry.id,
                    status,
                    error_message: entry.error_message,
                    sent_at: entry.sent_at.map(|dt| dt.to_rfc3339()),
                });
                self.send_json_response(stream, 200, &response)
            }
            None => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err("Action not found");
                self.send_json_response(stream, 404, &response)
            }
        }
    }

    /// List pending queue entries (local only).
    fn handle_list_queue(&self, stream: &mut TcpStream) -> Result<()> {
        let db = Database::open_at(self.db_path.clone())?;
        let entries = db.list_pending_queue()?;
        let api_keys = db.list_api_keys()?;

        // Build a map of key ID -> name
        let key_names: HashMap<String, String> = api_keys
            .into_iter()
            .map(|k| (k.id.clone(), k.name.clone()))
            .collect();

        let response_entries: Vec<QueueEntryResponse> = entries
            .into_iter()
            .map(|e| {
                let agent_name = key_names
                    .get(&e.api_key_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());

                QueueEntryResponse {
                    id: e.id,
                    channel: e.channel,
                    recipient_address: e.recipient_address,
                    recipient_name: e.recipient_name,
                    subject: e.subject,
                    body: e.body,
                    priority: e.priority,
                    status: e.status,
                    agent_context: e.agent_context.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: e.created_at.to_rfc3339(),
                    agent_name,
                }
            })
            .collect();

        let total = response_entries.len();
        let response = GatewayApiResponse::ok(QueueListResponse {
            entries: response_entries,
            total,
        });
        self.send_json_response(stream, 200, &response)
    }

    /// Approve and send a message (local only).
    fn handle_approve(&self, stream: &mut TcpStream, id: &str) -> Result<()> {
        let db = Database::open_at(self.db_path.clone())?;

        match db.get_queue_entry(id)? {
            Some(entry) => {
                if entry.status != "pending" && entry.status != "flagged" {
                    let response: GatewayApiResponse<()> =
                        GatewayApiResponse::err(format!("Cannot approve: status is {}", entry.status));
                    return self.send_json_response(stream, 400, &response);
                }

                // Mark as approved
                db.update_queue_status(id, "approved")?;

                // Execute send
                match execute::execute_send(&db, &entry) {
                    Ok(()) => {
                        db.mark_queue_sent(id)?;
                        let sent_at = chrono::Utc::now().to_rfc3339();
                        // Send webhook notification (non-blocking for errors)
                        let _ = webhook::notify_status_change(
                            &db,
                            &entry.api_key_id,
                            id,
                            "sent",
                            &entry.recipient_address,
                            &entry.channel,
                            Some(&sent_at),
                            None,
                        );
                        let response = GatewayApiResponse::ok(ActionStatusResponse {
                            action_id: id.to_string(),
                            status: QueueStatus::Sent,
                            error_message: None,
                            sent_at: Some(sent_at),
                        });
                        self.send_json_response(stream, 200, &response)
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        db.mark_queue_failed(id, &error_msg)?;
                        // Send webhook notification (non-blocking for errors)
                        let _ = webhook::notify_status_change(
                            &db,
                            &entry.api_key_id,
                            id,
                            "failed",
                            &entry.recipient_address,
                            &entry.channel,
                            None,
                            Some(&error_msg),
                        );
                        let response = GatewayApiResponse::ok(ActionStatusResponse {
                            action_id: id.to_string(),
                            status: QueueStatus::Failed,
                            error_message: Some(error_msg),
                            sent_at: None,
                        });
                        self.send_json_response(stream, 200, &response)
                    }
                }
            }
            None => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err("Message not found");
                self.send_json_response(stream, 404, &response)
            }
        }
    }

    /// Deny a message (local only).
    fn handle_deny(&self, stream: &mut TcpStream, id: &str) -> Result<()> {
        let db = Database::open_at(self.db_path.clone())?;

        match db.get_queue_entry(id)? {
            Some(entry) => {
                if entry.status != "pending" && entry.status != "flagged" {
                    let response: GatewayApiResponse<()> =
                        GatewayApiResponse::err(format!("Cannot deny: status is {}", entry.status));
                    return self.send_json_response(stream, 400, &response);
                }

                db.update_queue_status(id, "denied")?;

                // Send webhook notification (non-blocking for errors)
                let _ = webhook::notify_status_change(
                    &db,
                    &entry.api_key_id,
                    id,
                    "denied",
                    &entry.recipient_address,
                    &entry.channel,
                    None,
                    None,
                );

                let response = GatewayApiResponse::ok(ActionStatusResponse {
                    action_id: id.to_string(),
                    status: QueueStatus::Denied,
                    error_message: None,
                    sent_at: None,
                });
                self.send_json_response(stream, 200, &response)
            }
            None => {
                let response: GatewayApiResponse<()> =
                    GatewayApiResponse::err("Message not found");
                self.send_json_response(stream, 404, &response)
            }
        }
    }

    /// Authenticate request using X-Gateway-Key header.
    fn authenticate(&self, headers: &HashMap<String, String>) -> Result<crate::db::gateway::ApiKey> {
        let key = headers
            .get("x-gateway-key")
            .ok_or_else(|| anyhow!("Missing X-Gateway-Key header"))?;

        // Validate format
        keys::validate_key_format(key)?;

        // Hash and lookup
        let key_hash = keys::hash_key(key);
        let db = Database::open_at(self.db_path.clone())?;

        match db.find_api_key_by_hash(&key_hash)? {
            Some(api_key) => {
                if api_key.revoked_at.is_some() {
                    Err(anyhow!("API key has been revoked"))
                } else {
                    Ok(api_key)
                }
            }
            None => Err(anyhow!("Invalid API key")),
        }
    }

    fn send_response(&self, stream: &mut TcpStream, status: u16, message: &str) -> Result<()> {
        let status_text = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        };

        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status, status_text, message.len(), message
        );

        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    fn send_json_response<T: serde::Serialize>(
        &self,
        stream: &mut TcpStream,
        status: u16,
        body: &T,
    ) -> Result<()> {
        let status_text = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "Unknown",
        };

        let json_body = serde_json::to_string(body)?;

        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status, status_text, json_body.len(), json_body
        );

        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        Ok(())
    }
}

/// Check if a recipient address matches any pattern in the allowlist.
/// Supports:
/// - Exact match (case-insensitive for emails)
/// - Wildcard domain match: `*@domain.com` matches any email at that domain
/// - Phone normalization: strips spaces, dashes, parentheses for comparison
fn recipient_matches_allowlist(recipient: &str, patterns: &[String]) -> bool {
    let normalized_recipient = normalize_recipient(recipient);

    for pattern in patterns {
        if pattern_matches(&normalized_recipient, pattern) {
            return true;
        }
    }
    false
}

/// Normalize a recipient address for comparison.
/// - Emails: lowercase
/// - Phones: remove spaces, dashes, parentheses, dots
fn normalize_recipient(recipient: &str) -> String {
    let trimmed = recipient.trim();

    // Check if it looks like a phone number (starts with + or digit, contains mostly digits)
    if is_phone_number(trimmed) {
        normalize_phone(trimmed)
    } else {
        // Treat as email - lowercase
        trimmed.to_lowercase()
    }
}

/// Check if a string looks like a phone number.
fn is_phone_number(s: &str) -> bool {
    let first_char = s.chars().next();
    if first_char != Some('+') && !first_char.map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return false;
    }

    // Count digits vs other chars
    let digit_count = s.chars().filter(|c| c.is_ascii_digit()).count();
    let total_allowed = s.chars().filter(|c| c.is_ascii_digit() || *c == '+' || *c == '-' || *c == ' ' || *c == '(' || *c == ')' || *c == '.').count();

    // Must be mostly digits and have enough of them
    digit_count >= 7 && total_allowed == s.len()
}

/// Normalize a phone number by removing formatting characters.
fn normalize_phone(phone: &str) -> String {
    phone
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect()
}

/// Check if a normalized recipient matches a pattern.
fn pattern_matches(normalized_recipient: &str, pattern: &str) -> bool {
    let normalized_pattern = if is_phone_number(pattern) {
        normalize_phone(pattern)
    } else {
        pattern.to_lowercase()
    };

    // Wildcard domain match: *@domain.com
    if normalized_pattern.starts_with("*@") {
        let domain_suffix = &normalized_pattern[1..]; // "@domain.com"
        return normalized_recipient.ends_with(domain_suffix);
    }

    // Exact match
    normalized_recipient == normalized_pattern
}

#[cfg(test)]
mod allowlist_tests {
    use super::*;

    #[test]
    fn test_exact_email_match() {
        let patterns = vec!["john@example.com".to_string()];
        assert!(recipient_matches_allowlist("john@example.com", &patterns));
        assert!(recipient_matches_allowlist("JOHN@EXAMPLE.COM", &patterns));
        assert!(!recipient_matches_allowlist("jane@example.com", &patterns));
    }

    #[test]
    fn test_wildcard_domain_match() {
        let patterns = vec!["*@acme.com".to_string()];
        assert!(recipient_matches_allowlist("anyone@acme.com", &patterns));
        assert!(recipient_matches_allowlist("CEO@ACME.COM", &patterns));
        assert!(!recipient_matches_allowlist("someone@other.com", &patterns));
    }

    #[test]
    fn test_phone_normalization() {
        let patterns = vec!["+15551234567".to_string()];
        assert!(recipient_matches_allowlist("+15551234567", &patterns));
        assert!(recipient_matches_allowlist("+1 555 123 4567", &patterns));
        assert!(recipient_matches_allowlist("+1-555-123-4567", &patterns));
        assert!(recipient_matches_allowlist("+1 (555) 123-4567", &patterns));
        assert!(!recipient_matches_allowlist("+15559999999", &patterns));
    }

    #[test]
    fn test_multiple_patterns() {
        let patterns = vec![
            "john@example.com".to_string(),
            "*@acme.com".to_string(),
            "+15551234567".to_string(),
        ];
        assert!(recipient_matches_allowlist("john@example.com", &patterns));
        assert!(recipient_matches_allowlist("anyone@acme.com", &patterns));
        assert!(recipient_matches_allowlist("+1-555-123-4567", &patterns));
        assert!(!recipient_matches_allowlist("other@example.com", &patterns));
    }

    #[test]
    fn test_empty_allowlist() {
        let patterns: Vec<String> = vec![];
        // Empty patterns = nothing matches (caller should skip check for empty allowlist)
        assert!(!recipient_matches_allowlist("anyone@example.com", &patterns));
    }
}
