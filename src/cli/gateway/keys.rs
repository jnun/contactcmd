//! API key generation and validation for the gateway.

use anyhow::Result;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Prefix for gateway API keys.
const KEY_PREFIX: &str = "gw_";

/// Length of the random portion of the key (in bytes, hex-encoded = 2x chars).
const KEY_RANDOM_BYTES: usize = 24;

/// Generate a new API key.
///
/// Returns (full_key, key_hash, key_prefix) where:
/// - full_key: The complete key to give to the user (e.g., "gw_abc123...")
/// - key_hash: SHA-256 hash for storage
/// - key_prefix: First 8 chars for display (e.g., "gw_abc12")
pub fn generate_api_key() -> (String, String, String) {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..KEY_RANDOM_BYTES).map(|_| rng.gen()).collect();
    let random_hex = hex::encode(&random_bytes);

    let full_key = format!("{}{}", KEY_PREFIX, random_hex);
    let key_hash = hash_key(&full_key);
    let key_prefix = full_key.chars().take(11).collect(); // "gw_" + 8 chars

    (full_key, key_hash, key_prefix)
}

/// Hash a key for storage.
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate key format (starts with prefix, correct length).
pub fn validate_key_format(key: &str) -> Result<()> {
    if !key.starts_with(KEY_PREFIX) {
        anyhow::bail!("Invalid key format: must start with '{}'", KEY_PREFIX);
    }

    let expected_len = KEY_PREFIX.len() + (KEY_RANDOM_BYTES * 2);
    if key.len() != expected_len {
        anyhow::bail!(
            "Invalid key format: expected {} characters, got {}",
            expected_len,
            key.len()
        );
    }

    // Check that the random portion is valid hex
    let random_part = &key[KEY_PREFIX.len()..];
    if !random_part.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("Invalid key format: contains non-hex characters");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let (full_key, hash, prefix) = generate_api_key();

        // Check format
        assert!(full_key.starts_with("gw_"));
        assert_eq!(full_key.len(), 3 + 48); // "gw_" + 48 hex chars

        // Check hash is different from key
        assert_ne!(full_key, hash);
        assert_eq!(hash.len(), 64); // SHA-256 = 64 hex chars

        // Check prefix
        assert!(prefix.starts_with("gw_"));
        assert_eq!(prefix.len(), 11);
    }

    #[test]
    fn test_hash_key() {
        let hash1 = hash_key("gw_test123");
        let hash2 = hash_key("gw_test123");
        let hash3 = hash_key("gw_test456");

        // Same input = same hash
        assert_eq!(hash1, hash2);

        // Different input = different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_validate_key_format() {
        // Valid key
        let (key, _, _) = generate_api_key();
        assert!(validate_key_format(&key).is_ok());

        // Invalid prefix
        assert!(validate_key_format("bad_prefix123").is_err());

        // Wrong length
        assert!(validate_key_format("gw_tooshort").is_err());

        // Invalid hex
        let bad_key = format!("gw_{}", "g".repeat(48));
        assert!(validate_key_format(&bad_key).is_err());
    }
}
