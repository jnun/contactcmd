//! Messages integration for retrieving last iMessage from contacts.
//!
//! This module provides functionality to read the most recent text message
//! from/to a contact by querying the macOS Messages database.
//! On non-macOS platforms, this module provides stubs that return Ok(None).

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{get_last_message_for_phones, get_last_message_for_handles, get_messages_for_phones, get_messages_for_handles, run_messages, LastMessage, detect_service_for_phone, DetectedService, get_recent_message_handles, RecentHandle, phones_match_public};

#[cfg(not(target_os = "macos"))]
mod stub {
    use anyhow::Result;

    /// Represents a message from the Messages app
    #[derive(Debug, Clone)]
    pub struct LastMessage {
        pub text: String,
        pub date: chrono::DateTime<chrono::Local>,
        pub is_from_me: bool,
        pub handle: String,
    }

    /// Service type detected from chat history
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DetectedService {
        IMessage,
        Sms,
        Unknown,
    }

    /// Stub implementation for non-macOS platforms - always returns None
    pub fn get_last_message_for_phones(_phones: &[String]) -> Result<Option<LastMessage>> {
        Ok(None)
    }

    /// Stub implementation for non-macOS platforms - always returns None
    pub fn get_last_message_for_handles(_phones: &[String], _emails: &[String]) -> Result<Option<LastMessage>> {
        Ok(None)
    }

    /// Stub implementation for non-macOS platforms - always returns empty
    pub fn get_messages_for_phones(_phones: &[String], _limit: u32) -> Result<Vec<LastMessage>> {
        Ok(vec![])
    }

    /// Stub implementation for non-macOS platforms - always returns empty
    pub fn get_messages_for_handles(_phones: &[String], _emails: &[String], _limit: u32) -> Result<Vec<LastMessage>> {
        Ok(vec![])
    }

    /// Stub implementation for non-macOS platforms
    pub fn run_messages(_db: &crate::db::Database, _query: &str, _since: Option<&str>) -> Result<()> {
        println!("Messages search is only available on macOS.");
        Ok(())
    }

    /// Stub implementation for non-macOS platforms - always returns Unknown
    pub fn detect_service_for_phone(_phone: &str) -> Result<DetectedService> {
        Ok(DetectedService::Unknown)
    }

    /// A recent message handle with metadata
    #[derive(Debug, Clone)]
    pub struct RecentHandle {
        pub handle: String,
        pub last_message_date: chrono::DateTime<chrono::Local>,
        pub service: DetectedService,
    }

    /// Stub implementation for non-macOS platforms - always returns empty
    pub fn get_recent_message_handles(_days: u32) -> Result<Vec<RecentHandle>> {
        Ok(vec![])
    }

    /// Stub implementation for non-macOS platforms - always returns false
    pub fn phones_match_public(_phone1: &str, _phone2: &str) -> bool {
        false
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::{get_last_message_for_phones, get_last_message_for_handles, get_messages_for_phones, get_messages_for_handles, run_messages, LastMessage, detect_service_for_phone, DetectedService, get_recent_message_handles, RecentHandle, phones_match_public};
