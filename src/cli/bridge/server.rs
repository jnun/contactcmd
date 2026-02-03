//! HTTP server for receiving messages from Moltbot.

use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use super::signing::verify_signature;
use super::types::{BridgeApiResponse, HealthStatus, KillRequest, OutboundMessage};

/// Events emitted by the bridge server.
#[derive(Debug)]
pub enum BridgeEvent {
    /// Message received from Moltbot to send
    OutboundMessage(OutboundMessage),
    /// Kill request received
    Kill { reason: Option<String> },
    /// Server error
    Error(String),
}

/// HTTP server for receiving messages from Moltbot.
pub struct BridgeServer {
    port: u16,
    secret: String,
    token: Option<String>,
    start_time: Instant,
}

impl BridgeServer {
    /// Create a new bridge server.
    ///
    /// # Arguments
    /// * `port` - Port to listen on (default 9800)
    /// * `secret` - Shared HMAC secret
    /// * `token` - Optional authentication token
    pub fn new(port: u16, secret: String, token: Option<String>) -> Self {
        Self {
            port,
            secret,
            token,
            start_time: Instant::now(),
        }
    }

    /// Start the server, returning an event channel and a shutdown flag.
    ///
    /// The server runs in the current thread (blocking).
    pub fn start(
        &self,
        shutdown: Arc<AtomicBool>,
    ) -> Result<mpsc::UnboundedReceiver<BridgeEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))?;
        listener.set_nonblocking(true)?;

        println!("Bridge server listening on localhost:{}", self.port);

        while !shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Err(e) = self.handle_connection(stream, &tx) {
                        let _ = tx.send(BridgeEvent::Error(e.to_string()));
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection ready, sleep briefly
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    let _ = tx.send(BridgeEvent::Error(format!("Accept error: {}", e)));
                }
            }
        }

        println!("Bridge server shutting down");
        Ok(rx)
    }

    fn handle_connection(
        &self,
        mut stream: TcpStream,
        tx: &mpsc::UnboundedSender<BridgeEvent>,
    ) -> Result<()> {
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
        let mut headers = std::collections::HashMap::new();
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

        // Route request
        match (method, path) {
            ("GET", "/bridge/health") => self.handle_health(&mut stream),
            ("POST", "/bridge/outbound") => {
                self.handle_outbound(&mut stream, &headers, &body, tx)
            }
            ("POST", "/bridge/kill") => self.handle_kill(&mut stream, &headers, &body, tx),
            _ => self.send_response(&mut stream, 404, "Not Found"),
        }
    }

    fn handle_health(&self, stream: &mut TcpStream) -> Result<()> {
        let status = HealthStatus {
            status: "ok".to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            version: "1.0".to_string(),
        };

        let response: BridgeApiResponse<HealthStatus> = BridgeApiResponse::ok(status);
        self.send_json_response(stream, 200, &response)
    }

    fn handle_outbound(
        &self,
        stream: &mut TcpStream,
        headers: &std::collections::HashMap<String, String>,
        body: &[u8],
        tx: &mpsc::UnboundedSender<BridgeEvent>,
    ) -> Result<()> {
        // Verify signature
        if let Err(e) = self.verify_request(headers, body) {
            let response: BridgeApiResponse<()> = BridgeApiResponse::err(e.to_string());
            return self.send_json_response(stream, 401, &response);
        }

        // Parse message
        let message: OutboundMessage = match serde_json::from_slice(body) {
            Ok(m) => m,
            Err(e) => {
                let response: BridgeApiResponse<()> =
                    BridgeApiResponse::err(format!("Invalid message format: {}", e));
                return self.send_json_response(stream, 400, &response);
            }
        };

        // Send to event channel
        let _ = tx.send(BridgeEvent::OutboundMessage(message));

        let response: BridgeApiResponse<()> = BridgeApiResponse::ok(());
        self.send_json_response(stream, 200, &response)
    }

    fn handle_kill(
        &self,
        stream: &mut TcpStream,
        headers: &std::collections::HashMap<String, String>,
        body: &[u8],
        tx: &mpsc::UnboundedSender<BridgeEvent>,
    ) -> Result<()> {
        // Verify signature
        if let Err(e) = self.verify_request(headers, body) {
            let response: BridgeApiResponse<()> = BridgeApiResponse::err(e.to_string());
            return self.send_json_response(stream, 401, &response);
        }

        // Parse kill request
        let kill_req: KillRequest = serde_json::from_slice(body).unwrap_or(KillRequest { reason: None });

        // Send to event channel
        let _ = tx.send(BridgeEvent::Kill {
            reason: kill_req.reason,
        });

        let response: BridgeApiResponse<()> = BridgeApiResponse::ok(());
        self.send_json_response(stream, 200, &response)
    }

    fn verify_request(
        &self,
        headers: &std::collections::HashMap<String, String>,
        body: &[u8],
    ) -> Result<()> {
        // Check token if configured
        if let Some(ref expected_token) = self.token {
            let token = headers
                .get("x-bridge-token")
                .ok_or_else(|| anyhow!("Missing X-Bridge-Token header"))?;
            if token != expected_token {
                return Err(anyhow!("Invalid token"));
            }
        }

        // Verify HMAC signature
        let timestamp = headers
            .get("x-bridge-timestamp")
            .ok_or_else(|| anyhow!("Missing X-Bridge-Timestamp header"))?;

        let signature = headers
            .get("x-bridge-signature")
            .ok_or_else(|| anyhow!("Missing X-Bridge-Signature header"))?;

        verify_signature(&self.secret, timestamp, body, signature)
    }

    fn send_response(&self, stream: &mut TcpStream, status: u16, message: &str) -> Result<()> {
        let status_text = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
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
            404 => "Not Found",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = BridgeServer::new(9800, "test-secret".to_string(), None);
        assert_eq!(server.port, 9800);
    }

    #[test]
    fn test_server_with_token() {
        let server = BridgeServer::new(
            9800,
            "test-secret".to_string(),
            Some("my-token".to_string()),
        );
        assert!(server.token.is_some());
    }
}
