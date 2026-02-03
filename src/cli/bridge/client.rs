//! HTTP client for communicating with Moltbot.

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use std::time::Duration;

use super::signing::{compute_signature, current_timestamp};
use super::types::{BridgeApiResponse, HandshakeRequest, HandshakeResponse, HealthStatus, InboundMessage};

/// HTTP client for sending messages to Moltbot.
pub struct BridgeClient {
    client: Client,
    base_url: String,
    secret: String,
    token: Option<String>,
}

impl BridgeClient {
    /// Create a new bridge client.
    ///
    /// # Arguments
    /// * `port` - Moltbot's listening port
    /// * `secret` - Shared HMAC secret
    /// * `token` - Optional authentication token
    pub fn new(port: u16, secret: String, token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: format!("http://localhost:{}", port),
            secret,
            token,
        })
    }

    /// Perform initial handshake with Moltbot.
    ///
    /// Returns the session token if accepted.
    pub fn handshake(&mut self) -> Result<String> {
        let request = HandshakeRequest::default();
        let body = serde_json::to_vec(&request)?;
        let timestamp = current_timestamp();
        let signature = compute_signature(&self.secret, &timestamp, &body);

        let mut req = self
            .client
            .post(format!("{}/bridge/handshake", self.base_url))
            .header("Content-Type", "application/json")
            .header("X-Bridge-Timestamp", &timestamp)
            .header("X-Bridge-Signature", &signature);

        if let Some(ref token) = self.token {
            req = req.header("X-Bridge-Token", token);
        }

        let response = req.body(body).send()?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Handshake failed with status: {}",
                response.status()
            ));
        }

        let resp: BridgeApiResponse<HandshakeResponse> = response.json()?;

        if !resp.success {
            return Err(anyhow!(
                "Handshake rejected: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            ));
        }

        let data = resp.data.ok_or_else(|| anyhow!("Missing handshake response data"))?;

        if !data.accepted {
            return Err(anyhow!(
                "Handshake not accepted: {}",
                data.error.unwrap_or_else(|| "unknown reason".to_string())
            ));
        }

        let session_token = data
            .session_token
            .ok_or_else(|| anyhow!("No session token in handshake response"))?;

        self.token = Some(session_token.clone());
        Ok(session_token)
    }

    /// Send an inbound message to Moltbot.
    pub fn send_inbound(&self, message: &InboundMessage) -> Result<()> {
        let body = serde_json::to_vec(message)?;
        let timestamp = current_timestamp();
        let signature = compute_signature(&self.secret, &timestamp, &body);

        let mut req = self
            .client
            .post(format!("{}/bridge/inbound", self.base_url))
            .header("Content-Type", "application/json")
            .header("X-Bridge-Timestamp", &timestamp)
            .header("X-Bridge-Signature", &signature);

        if let Some(ref token) = self.token {
            req = req.header("X-Bridge-Token", token);
        }

        let response = req.body(body).send()?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to send inbound message: {}",
                response.status()
            ));
        }

        let resp: BridgeApiResponse<()> = response.json()?;

        if !resp.success {
            return Err(anyhow!(
                "Inbound message rejected: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            ));
        }

        Ok(())
    }

    /// Check if Moltbot is reachable.
    pub fn health_check(&self) -> Result<HealthStatus> {
        let response = self
            .client
            .get(format!("{}/bridge/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()?;

        if !response.status().is_success() {
            return Err(anyhow!("Health check failed: {}", response.status()));
        }

        let resp: BridgeApiResponse<HealthStatus> = response.json()?;

        if !resp.success {
            return Err(anyhow!(
                "Health check error: {}",
                resp.error.unwrap_or_else(|| "unknown".to_string())
            ));
        }

        resp.data
            .ok_or_else(|| anyhow!("Missing health status data"))
    }

    /// Update the authentication token.
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Get the current token.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BridgeClient::new(9801, "test-secret".to_string(), None);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(client.token().is_none());
    }

    #[test]
    fn test_client_with_token() {
        let client =
            BridgeClient::new(9801, "test-secret".to_string(), Some("my-token".to_string()))
                .unwrap();
        assert_eq!(client.token(), Some("my-token"));
    }
}
