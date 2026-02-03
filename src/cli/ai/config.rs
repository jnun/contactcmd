//! AI configuration management
//!
//! Handles loading and storing AI provider settings from environment
//! variables and database settings.

use crate::db::Database;
use anyhow::Result;
use std::env;

// Settings keys for database storage
pub const SETTING_AI_PROVIDER: &str = "ai_provider";
pub const SETTING_AI_API_KEY: &str = "ai_api_key";
pub const SETTING_AI_API_URL: &str = "ai_api_url";
pub const SETTING_AI_API_ENDPOINT: &str = "ai_api_endpoint";
pub const SETTING_AI_MODEL: &str = "ai_model";
pub const SETTING_AI_LOCAL_MODEL: &str = "ai_local_model";

// Environment variable names
const ENV_AI_API_KEY: &str = "AI_API_KEY";
const ENV_AI_API_URL: &str = "AI_API_URL";
const ENV_AI_API_ENDPOINT: &str = "AI_API_ENDPOINT";

/// Type of AI provider
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AiProviderType {
    #[default]
    None,
    Remote,
    #[cfg(feature = "local-ai")]
    Local,
}

impl AiProviderType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "remote" => Self::Remote,
            #[cfg(feature = "local-ai")]
            "local" => Self::Local,
            _ => Self::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Remote => "remote",
            #[cfg(feature = "local-ai")]
            Self::Local => "local",
        }
    }
}

/// Supported local models (when local-ai feature is enabled)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(non_camel_case_types)]
pub enum LocalModelId {
    #[default]
    Qwen3_4B,
    Gemma3n_E4B,
    Llama31_8B,
    Magistral_Small_24B,
}

impl LocalModelId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "qwen3-4b" | "qwen3_4b" => Some(Self::Qwen3_4B),
            "gemma3n-e4b" | "gemma3n_e4b" => Some(Self::Gemma3n_E4B),
            "llama31-8b" | "llama31_8b" | "llama3.1-8b" => Some(Self::Llama31_8B),
            "magistral-small-24b" | "magistral_small_24b" => Some(Self::Magistral_Small_24B),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Qwen3_4B => "qwen3-4b",
            Self::Gemma3n_E4B => "gemma3n-e4b",
            Self::Llama31_8B => "llama31-8b",
            Self::Magistral_Small_24B => "magistral-small-24b",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Qwen3_4B => "Qwen3 4B (Default - 2.75 GB)",
            Self::Gemma3n_E4B => "Gemma 3n E4B (5.5 GB)",
            Self::Llama31_8B => "Llama 3.1 8B (6-7 GB)",
            Self::Magistral_Small_24B => "Magistral Small 24B (13 GB)",
        }
    }

    pub fn min_ram_gb(&self) -> u64 {
        match self {
            Self::Qwen3_4B => 4,
            Self::Gemma3n_E4B => 8,
            Self::Llama31_8B => 8,
            Self::Magistral_Small_24B => 16,
        }
    }

    pub fn all() -> &'static [LocalModelId] {
        &[
            Self::Qwen3_4B,
            Self::Gemma3n_E4B,
            Self::Llama31_8B,
            Self::Magistral_Small_24B,
        ]
    }
}

/// Configuration for the AI provider
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider_type: AiProviderType,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub api_endpoint: Option<String>,
    pub model: Option<String>,
    pub local_model: Option<LocalModelId>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider_type: AiProviderType::None,
            api_key: None,
            api_url: None,
            api_endpoint: None,
            model: None,
            local_model: None,
        }
    }
}

impl AiConfig {
    /// Load configuration from environment variables and database settings.
    /// Environment variables take precedence over database settings.
    pub fn load(db: &Database) -> Result<Self> {
        // Load provider type from database
        let provider_type = db
            .get_setting(SETTING_AI_PROVIDER)?
            .map(|s| AiProviderType::from_str(&s))
            .unwrap_or_default();

        // API key: env var takes precedence
        let api_key = env::var(ENV_AI_API_KEY)
            .ok()
            .or_else(|| db.get_setting(SETTING_AI_API_KEY).ok().flatten());

        // API URL: env var takes precedence
        let api_url = env::var(ENV_AI_API_URL)
            .ok()
            .or_else(|| db.get_setting(SETTING_AI_API_URL).ok().flatten());

        // API endpoint: env var takes precedence
        let api_endpoint = env::var(ENV_AI_API_ENDPOINT)
            .ok()
            .or_else(|| db.get_setting(SETTING_AI_API_ENDPOINT).ok().flatten());

        // Model name from database
        let model = db.get_setting(SETTING_AI_MODEL)?;

        // Local model selection from database
        let local_model = db
            .get_setting(SETTING_AI_LOCAL_MODEL)?
            .and_then(|s| LocalModelId::from_str(&s));

        Ok(Self {
            provider_type,
            api_key,
            api_url,
            api_endpoint,
            model,
            local_model,
        })
    }

    /// Check if AI is configured and ready to use
    pub fn is_configured(&self) -> bool {
        match self.provider_type {
            AiProviderType::None => false,
            AiProviderType::Remote => self.api_key.is_some(),
            #[cfg(feature = "local-ai")]
            AiProviderType::Local => self.local_model.is_some(),
        }
    }

    /// Get the effective API URL (with default)
    pub fn effective_api_url(&self) -> &str {
        self.api_url
            .as_deref()
            .unwrap_or("https://api.openai.com")
    }

    /// Get the effective API endpoint (with default)
    pub fn effective_api_endpoint(&self) -> &str {
        self.api_endpoint
            .as_deref()
            .unwrap_or("/v1/chat/completions")
    }

    /// Get the effective model name (with default)
    pub fn effective_model(&self) -> &str {
        self.model.as_deref().unwrap_or("gpt-4o-mini")
    }

    /// Save the current configuration to the database
    pub fn save(&self, db: &Database) -> Result<()> {
        db.set_setting(SETTING_AI_PROVIDER, self.provider_type.as_str())?;

        if let Some(ref key) = self.api_key {
            db.set_setting(SETTING_AI_API_KEY, key)?;
        }

        if let Some(ref url) = self.api_url {
            db.set_setting(SETTING_AI_API_URL, url)?;
        }

        if let Some(ref endpoint) = self.api_endpoint {
            db.set_setting(SETTING_AI_API_ENDPOINT, endpoint)?;
        }

        if let Some(ref model) = self.model {
            db.set_setting(SETTING_AI_MODEL, model)?;
        }

        if let Some(local_model) = self.local_model {
            db.set_setting(SETTING_AI_LOCAL_MODEL, local_model.as_str())?;
        }

        Ok(())
    }

    /// Clear all AI configuration from the database
    pub fn clear(db: &Database) -> Result<()> {
        let _ = db.delete_setting(SETTING_AI_PROVIDER);
        let _ = db.delete_setting(SETTING_AI_API_KEY);
        let _ = db.delete_setting(SETTING_AI_API_URL);
        let _ = db.delete_setting(SETTING_AI_API_ENDPOINT);
        let _ = db.delete_setting(SETTING_AI_MODEL);
        let _ = db.delete_setting(SETTING_AI_LOCAL_MODEL);
        Ok(())
    }
}

/// Check available system RAM in GB
pub fn get_available_ram_gb() -> u64 {
    use sysinfo::System;
    let sys = System::new_all();
    sys.total_memory() / (1024 * 1024 * 1024)
}

/// Check if system has enough RAM for a model
pub fn check_ram_for_model(model: LocalModelId) -> (bool, u64, u64) {
    let available = get_available_ram_gb();
    let required = model.min_ram_gb();
    (available >= required, available, required)
}
