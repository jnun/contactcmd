//! Messages integration for retrieving last iMessage from contacts.
//!
//! This module provides functionality to read the most recent text message
//! from/to a contact by querying the macOS Messages database.
//! On non-macOS platforms, this module provides stubs that return Ok(None).

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{get_last_message_for_phones, get_last_message_for_handles, get_messages_for_phones, get_messages_for_handles, search_messages, run_messages, LastMessage};

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

    /// Stub implementation for non-macOS platforms - always returns empty
    pub fn search_messages(_terms: &[&str], _limit: u32) -> Result<Vec<LastMessage>> {
        Ok(vec![])
    }

    /// Stub implementation for non-macOS platforms
    pub fn run_messages(_db: &crate::db::Database, _query: &str) -> Result<()> {
        println!("Messages search is only available on macOS.");
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::{get_last_message_for_phones, get_last_message_for_handles, get_messages_for_phones, get_messages_for_handles, search_messages, run_messages, LastMessage};
