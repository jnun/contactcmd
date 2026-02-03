//! Local model definitions and registry
//!
//! Defines available local models with their download URLs, sizes, and configuration.

use super::config::LocalModelId;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Information about a local model
#[derive(Debug, Clone)]
pub struct LocalModel {
    pub id: LocalModelId,
    pub name: &'static str,
    pub description: &'static str,
    /// Download URL for the GGUF model file
    pub download_url: &'static str,
    /// Expected file size in bytes (for download progress)
    pub file_size_bytes: u64,
    /// Model file size description
    pub size_description: &'static str,
    /// Minimum RAM required in GB
    pub min_ram_gb: u64,
    /// Recommended context length
    pub context_length: u32,
    /// Whether this model requires a warning before download
    pub requires_warning: bool,
}

/// Registry of all supported local models
pub static MODEL_REGISTRY: LazyLock<HashMap<LocalModelId, LocalModel>> = LazyLock::new(|| {
    let mut registry = HashMap::new();

    // Qwen3 4B - Default, lightest model
    registry.insert(
        LocalModelId::Qwen3_4B,
        LocalModel {
            id: LocalModelId::Qwen3_4B,
            name: "Qwen3 4B",
            description: "Lightweight and fast, works on most machines",
            download_url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
            file_size_bytes: 2_200_000_000, // ~2.2 GB
            size_description: "~2.75 GB",
            min_ram_gb: 4,
            context_length: 4096,
            requires_warning: false,
        },
    );

    // Gemma 3n E4B
    registry.insert(
        LocalModelId::Gemma3n_E4B,
        LocalModel {
            id: LocalModelId::Gemma3n_E4B,
            name: "Gemma 3n E4B",
            description: "Good quality, vision-capable model",
            download_url: "https://huggingface.co/google/gemma-2-9b-it-GGUF/resolve/main/gemma-2-9b-it-Q4_K_M.gguf",
            file_size_bytes: 5_500_000_000, // ~5.5 GB
            size_description: "~5.5 GB",
            min_ram_gb: 8,
            context_length: 8192,
            requires_warning: false,
        },
    );

    // Llama 3.1 8B
    registry.insert(
        LocalModelId::Llama31_8B,
        LocalModel {
            id: LocalModelId::Llama31_8B,
            name: "Llama 3.1 8B",
            description: "Popular, well-supported model",
            download_url: "https://huggingface.co/lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf",
            file_size_bytes: 4_920_000_000, // ~4.9 GB
            size_description: "~6-7 GB",
            min_ram_gb: 8,
            context_length: 8192,
            requires_warning: false,
        },
    );

    // Magistral Small 24B
    registry.insert(
        LocalModelId::Magistral_Small_24B,
        LocalModel {
            id: LocalModelId::Magistral_Small_24B,
            name: "Magistral Small 24B",
            description: "High quality, requires significant RAM",
            download_url: "https://huggingface.co/mistralai/Mistral-Small-3.1-24B-Instruct-2503-GGUF/resolve/main/Mistral-Small-3.1-24B-Instruct-2503-Q4_K_M.gguf",
            file_size_bytes: 13_000_000_000, // ~13 GB
            size_description: "~13 GB",
            min_ram_gb: 16,
            context_length: 32768,
            requires_warning: true,
        },
    );

    registry
});

impl LocalModel {
    /// Get the model info by ID
    pub fn get(id: LocalModelId) -> Option<&'static LocalModel> {
        MODEL_REGISTRY.get(&id)
    }

    /// Get all available models
    pub fn all() -> Vec<&'static LocalModel> {
        LocalModelId::all()
            .iter()
            .filter_map(|id| MODEL_REGISTRY.get(id))
            .collect()
    }

    /// Get the filename from the download URL
    pub fn filename(&self) -> &str {
        self.download_url
            .rsplit('/')
            .next()
            .unwrap_or("model.gguf")
    }

    /// Get the local file path for this model
    pub fn local_path(&self) -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("contactcmd");
        path.push("models");
        path.push(self.filename());
        path
    }

    /// Check if the model is already downloaded
    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_models_in_registry() {
        for id in LocalModelId::all() {
            assert!(
                MODEL_REGISTRY.contains_key(id),
                "Model {:?} not in registry",
                id
            );
        }
    }

    #[test]
    fn test_model_filename() {
        let model = LocalModel::get(LocalModelId::Qwen3_4B).unwrap();
        assert!(model.filename().ends_with(".gguf"));
    }

    #[test]
    fn test_default_model_no_warning() {
        let model = LocalModel::get(LocalModelId::Qwen3_4B).unwrap();
        assert!(!model.requires_warning);
    }

    #[test]
    fn test_large_model_warning() {
        let model = LocalModel::get(LocalModelId::Magistral_Small_24B).unwrap();
        assert!(model.requires_warning);
    }
}
