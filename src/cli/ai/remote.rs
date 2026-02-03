//! Remote AI provider implementation
//!
//! Implements the AiProvider trait for OpenAI-compatible HTTP APIs.
//! Works with OpenAI, Groq, Together AI, local vLLM, etc.

use super::{
    provider::AiProvider, AiConfig, AiResponse, AiTool, ChatMessage, FunctionCall, ToolCall,
};
use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Remote AI provider using OpenAI-compatible API
pub struct RemoteProvider {
    client: Client,
    api_url: String,
    api_endpoint: String,
    api_key: String,
    model: String,
}

impl RemoteProvider {
    /// Create a new remote provider from configuration
    pub fn new(config: &AiConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| anyhow!("API key not configured"))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            api_url: config.effective_api_url().to_string(),
            api_endpoint: config.effective_api_endpoint().to_string(),
            api_key,
            model: config.effective_model().to_string(),
        })
    }

    /// Build the full API URL
    fn full_url(&self) -> String {
        format!("{}{}", self.api_url, self.api_endpoint)
    }
}

impl AiProvider for RemoteProvider {
    fn complete(&self, messages: &[ChatMessage], tools: &[AiTool]) -> Result<AiResponse> {
        let request = CompletionRequest {
            model: &self.model,
            messages,
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.iter().map(OpenAiTool::from).collect())
            },
            tool_choice: if tools.is_empty() {
                None
            } else {
                Some("auto")
            },
        };

        let response = self
            .client
            .post(self.full_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, body));
        }

        let completion: CompletionResponse = response.json()?;
        let choice = completion
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No completion choices returned"))?;

        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ToolCall {
                id: tc.id,
                call_type: tc.r#type,
                function: FunctionCall {
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                },
            })
            .collect::<Vec<_>>();

        let is_complete = choice.finish_reason.as_deref() == Some("stop");

        Ok(AiResponse {
            content: choice.message.content,
            tool_calls,
            is_complete,
            finish_reason: choice.finish_reason,
        })
    }

    fn name(&self) -> &str {
        "Remote API"
    }

    fn is_ready(&self) -> bool {
        !self.api_key.is_empty()
    }
}

// OpenAI API request/response types

#[derive(Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
}

#[derive(Serialize)]
struct OpenAiTool {
    r#type: &'static str,
    function: OpenAiFunction,
}

#[derive(Serialize)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

impl From<&AiTool> for OpenAiTool {
    fn from(tool: &AiTool) -> Self {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &tool.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), serde_json::json!(param.param_type));
            prop.insert(
                "description".to_string(),
                serde_json::json!(param.description),
            );
            if let Some(ref enum_values) = param.enum_values {
                prop.insert("enum".to_string(), serde_json::json!(enum_values));
            }
            properties.insert(param.name.clone(), serde_json::Value::Object(prop));

            if param.required {
                required.push(param.name.clone());
            }
        }

        let parameters = serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required,
        });

        Self {
            r#type: "function",
            function: OpenAiFunction {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters,
            },
        }
    }
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ResponseToolCall>>,
}

#[derive(Deserialize)]
struct ResponseToolCall {
    id: String,
    r#type: String,
    function: ResponseFunction,
}

#[derive(Deserialize)]
struct ResponseFunction {
    name: String,
    arguments: String,
}
