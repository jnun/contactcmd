//! HMAC-SHA256 request signing and verification.

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Maximum allowed timestamp drift in seconds (5 minutes).
const MAX_TIMESTAMP_DRIFT_SECS: u64 = 300;

/// Compute HMAC-SHA256 signature for a request.
///
/// The signature is computed over: `timestamp.body`
///
/// # Arguments
/// * `secret` - Shared secret key
/// * `timestamp` - Unix timestamp as string
/// * `body` - Request body bytes
///
/// # Returns
/// Hex-encoded signature string
pub fn compute_signature(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");

    // Sign: timestamp.body
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);

    hex::encode(mac.finalize().into_bytes())
}

/// Verify HMAC-SHA256 signature for a request.
///
/// Uses constant-time comparison to prevent timing attacks.
/// Also validates timestamp is within acceptable drift window.
///
/// # Arguments
/// * `secret` - Shared secret key
/// * `timestamp` - Unix timestamp as string
/// * `body` - Request body bytes
/// * `signature` - Hex-encoded signature to verify
///
/// # Returns
/// Ok(()) if valid, Err with reason otherwise
pub fn verify_signature(secret: &str, timestamp: &str, body: &[u8], signature: &str) -> Result<()> {
    // Validate timestamp is within acceptable range
    let ts: u64 = timestamp
        .parse()
        .map_err(|_| anyhow!("Invalid timestamp format"))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("System time error"))?
        .as_secs();

    let drift = if now > ts { now - ts } else { ts - now };

    if drift > MAX_TIMESTAMP_DRIFT_SECS {
        return Err(anyhow!(
            "Timestamp too far from current time (drift: {}s, max: {}s)",
            drift,
            MAX_TIMESTAMP_DRIFT_SECS
        ));
    }

    // Compute expected signature
    let expected = compute_signature(secret, timestamp, body);

    // Constant-time comparison
    if constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
        Ok(())
    } else {
        Err(anyhow!("Signature verification failed"))
    }
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Get current Unix timestamp as string.
pub fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
        .to_string()
}

/// Generate a random shared secret.
pub fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Generate a random token.
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_signature() {
        let secret = "test-secret";
        let timestamp = "1700000000";
        let body = b"hello world";

        let sig1 = compute_signature(secret, timestamp, body);
        let sig2 = compute_signature(secret, timestamp, body);

        // Same inputs should produce same signature
        assert_eq!(sig1, sig2);

        // Different secret should produce different signature
        let sig3 = compute_signature("other-secret", timestamp, body);
        assert_ne!(sig1, sig3);

        // Different body should produce different signature
        let sig4 = compute_signature(secret, timestamp, b"other body");
        assert_ne!(sig1, sig4);
    }

    #[test]
    fn test_verify_signature_valid() {
        let secret = "test-secret";
        let timestamp = current_timestamp();
        let body = b"hello world";

        let signature = compute_signature(secret, &timestamp, body);
        let result = verify_signature(secret, &timestamp, body, &signature);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let secret = "test-secret";
        let timestamp = current_timestamp();
        let body = b"hello world";

        let result = verify_signature(secret, &timestamp, body, "invalid-signature");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature_old_timestamp() {
        let secret = "test-secret";
        // 10 minutes ago
        let old_timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 600)
            .to_string();
        let body = b"hello world";

        let signature = compute_signature(secret, &old_timestamp, body);
        let result = verify_signature(secret, &old_timestamp, body, &signature);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Timestamp"));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"x"));
    }

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 64); // 32 bytes = 64 hex chars
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token() {
        let token = generate_token();
        assert_eq!(token.len(), 32); // 16 bytes = 32 hex chars
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
