//! Local AI provider implementation
//!
//! Implements the AiProvider trait using llama-cpp-2 for local model inference.
//! This module is only available when the `local-ai` feature is enabled.

use super::{
    config::LocalModelId,
    models::{LocalModel, MODEL_REGISTRY},
    provider::AiProvider,
    AiConfig, AiResponse, AiTool, ChatMessage, FunctionCall, MessageRole, ToolCall,
};
use anyhow::{anyhow, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use std::io::{stdout, Write};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Local AI provider using llama-cpp-2
pub struct LocalProvider {
    model: Arc<LlamaModel>,
    backend: Arc<LlamaBackend>,
    model_id: LocalModelId,
}

impl LocalProvider {
    /// Create a new local provider from configuration
    pub fn new(config: &AiConfig) -> Result<Self> {
        let model_id = config
            .local_model
            .ok_or_else(|| anyhow!("No local model configured"))?;

        let model_info = LocalModel::get(model_id)
            .ok_or_else(|| anyhow!("Unknown model: {:?}", model_id))?;

        if !model_info.is_downloaded() {
            return Err(anyhow!(
                "Model not downloaded. Please run setup to download {}",
                model_info.name
            ));
        }

        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default();
        let model_path = model_info.local_path();
        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)?;

        Ok(Self {
            model: Arc::new(model),
            backend: Arc::new(backend),
            model_id,
        })
    }

    /// Convert messages to a prompt string
    fn messages_to_prompt(&self, messages: &[ChatMessage], tools: &[AiTool]) -> String {
        let mut prompt = String::new();

        // Add tool descriptions to system message
        let tool_info = if !tools.is_empty() {
            let mut info = String::from("\n\nAvailable tools:\n");
            for tool in tools {
                info.push_str(&format!("- {}: {}\n", tool.name, tool.description));
                if !tool.parameters.is_empty() {
                    info.push_str("  Parameters:\n");
                    for param in &tool.parameters {
                        let req = if param.required { "required" } else { "optional" };
                        info.push_str(&format!(
                            "    - {} ({}): {}\n",
                            param.name, req, param.description
                        ));
                    }
                }
            }
            info.push_str("\nTo use a tool, respond with:\n");
            info.push_str("<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n");
            info
        } else {
            String::new()
        };

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    prompt.push_str("<|system|>\n");
                    if let Some(ref content) = msg.content {
                        prompt.push_str(content);
                    }
                    prompt.push_str(&tool_info);
                    prompt.push_str("\n<|end|>\n");
                }
                MessageRole::User => {
                    prompt.push_str("<|user|>\n");
                    if let Some(ref content) = msg.content {
                        prompt.push_str(content);
                    }
                    prompt.push_str("\n<|end|>\n");
                }
                MessageRole::Assistant => {
                    prompt.push_str("<|assistant|>\n");
                    if let Some(ref content) = msg.content {
                        prompt.push_str(content);
                    }
                    prompt.push_str("\n<|end|>\n");
                }
                MessageRole::Tool => {
                    prompt.push_str("<|tool|>\n");
                    if let Some(ref content) = msg.content {
                        prompt.push_str(content);
                    }
                    prompt.push_str("\n<|end|>\n");
                }
            }
        }

        // Start assistant turn
        prompt.push_str("<|assistant|>\n");

        prompt
    }

    /// Parse tool calls from response
    fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut tool_calls = Vec::new();

        // Look for <tool_call>...</tool_call> patterns
        let mut remaining = response;
        while let Some(start) = remaining.find("<tool_call>") {
            if let Some(end) = remaining[start..].find("</tool_call>") {
                let json_str = &remaining[start + 11..start + end].trim();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(name), Some(args)) = (
                        parsed.get("name").and_then(|v| v.as_str()),
                        parsed.get("arguments"),
                    ) {
                        tool_calls.push(ToolCall {
                            id: format!("call_{}", tool_calls.len()),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: name.to_string(),
                                arguments: args.to_string(),
                            },
                        });
                    }
                }
                remaining = &remaining[start + end + 12..];
            } else {
                break;
            }
        }

        tool_calls
    }

    /// Remove tool call tags from response
    fn clean_response(&self, response: &str) -> String {
        let mut clean = response.to_string();

        // Remove tool call blocks
        while let Some(start) = clean.find("<tool_call>") {
            if let Some(end) = clean[start..].find("</tool_call>") {
                clean = format!("{}{}", &clean[..start], &clean[start + end + 12..]);
            } else {
                break;
            }
        }

        clean.trim().to_string()
    }
}

impl AiProvider for LocalProvider {
    fn complete(&self, messages: &[ChatMessage], tools: &[AiTool]) -> Result<AiResponse> {
        let prompt = self.messages_to_prompt(messages, tools);

        let model_info = LocalModel::get(self.model_id).unwrap();
        let ctx_size = NonZeroU32::new(model_info.context_length).unwrap();

        let ctx_params = LlamaContextParams::default().with_n_ctx(ctx_size);
        let mut ctx = self.model.new_context(&self.backend, ctx_params)?;

        // Tokenize prompt
        let tokens = self
            .model
            .str_to_token(&prompt, llama_cpp_2::model::AddBos::Always)?;

        // Create batch
        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        for (i, token) in tokens.iter().enumerate() {
            batch.add(*token, i as i32, &[0], i == tokens.len() - 1)?;
        }

        // Decode initial prompt
        ctx.decode(&mut batch)?;

        // Generate response
        let mut response = String::new();
        let max_tokens = 2048;
        let mut n_cur = batch.n_tokens();

        for _ in 0..max_tokens {
            let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
            let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);

            // Sample token with temperature
            let new_token_id = ctx.sample_token_softmax(&mut candidates_p);

            // Check for end of generation
            if self.model.is_eog_token(new_token_id) {
                break;
            }

            let token_str = self.model.token_to_str(new_token_id, Default::default())?;
            response.push_str(&token_str);

            // Check for end markers
            if response.contains("<|end|>") || response.contains("<|endoftext|>") {
                break;
            }

            // Prepare next batch
            batch.clear();
            batch.add(new_token_id, n_cur, &[0], true)?;
            n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        // Clean up response
        let response = response
            .replace("<|end|>", "")
            .replace("<|endoftext|>", "")
            .trim()
            .to_string();

        // Parse tool calls
        let tool_calls = self.parse_tool_calls(&response);
        let clean_response = self.clean_response(&response);

        if !tool_calls.is_empty() {
            Ok(AiResponse::with_tool_calls(tool_calls))
        } else {
            Ok(AiResponse::text(clean_response))
        }
    }

    fn name(&self) -> &str {
        LocalModel::get(self.model_id)
            .map(|m| m.name)
            .unwrap_or("Local Model")
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Download a model with progress display
pub fn download_model(model_id: LocalModelId) -> Result<()> {
    use reqwest::blocking::Client;
    use std::fs::{self, File};
    use std::io::Read;

    let model_info =
        LocalModel::get(model_id).ok_or_else(|| anyhow!("Unknown model: {:?}", model_id))?;

    let path = model_info.local_path();

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Downloading {} ({})", model_info.name, model_info.size_description);
    println!("URL: {}", model_info.download_url);
    println!("To: {}\n", path.display());

    let client = Client::builder()
        .timeout(None) // No timeout for large downloads
        .build()?;

    let mut response = client.get(model_info.download_url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response
        .content_length()
        .unwrap_or(model_info.file_size_bytes);

    let mut file = File::create(&path)?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        // Progress display
        let percent = (downloaded as f64 / total_size as f64 * 100.0).min(100.0);
        let mb_downloaded = downloaded as f64 / 1_000_000.0;
        let mb_total = total_size as f64 / 1_000_000.0;

        print!(
            "\r[{:>3.0}%] {:.1} MB / {:.1} MB",
            percent, mb_downloaded, mb_total
        );
        stdout().flush()?;
    }

    println!("\nDownload complete!");

    Ok(())
}

/// Delete a downloaded model
pub fn delete_model(model_id: LocalModelId) -> Result<()> {
    let model_info =
        LocalModel::get(model_id).ok_or_else(|| anyhow!("Unknown model: {:?}", model_id))?;

    let path = model_info.local_path();

    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("Deleted {}", model_info.name);
    } else {
        println!("{} is not downloaded", model_info.name);
    }

    Ok(())
}
