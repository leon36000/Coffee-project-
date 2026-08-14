use async_trait::async_trait;
use hc_domain::{ToolCall, ToolResult};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", content = "value", rename_all = "snake_case")]
pub enum ModelMessage {
    System(String),
    User(String),
    AssistantText(String),
    AssistantToolCalls(Vec<ToolCall>),
    ToolResult(ToolResult),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub messages: Vec<ModelMessage>,
}

impl ModelRequest {
    pub fn new(messages: Vec<ModelMessage>) -> Self {
        Self { messages }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(vec![ModelMessage::User(content.into())])
    }

    pub fn with_tool_result(user: impl Into<String>, result: ToolResult) -> Self {
        Self::new(vec![
            ModelMessage::User(user.into()),
            ModelMessage::ToolResult(result),
        ])
    }

    fn latest_tool_result(&self) -> Option<&ToolResult> {
        self.messages
            .iter()
            .rev()
            .find_map(|message| match message {
                ModelMessage::ToolResult(result) => Some(result),
                _ => None,
            })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ModelOutput {
    ToolCalls(Vec<ToolCall>),
    FinalText(String),
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn next_turn(&self, request: ModelRequest) -> Result<ModelOutput, ModelError>;
}

#[derive(Default)]
pub struct DeterministicProvider;

#[async_trait]
impl ModelProvider for DeterministicProvider {
    async fn next_turn(&self, request: ModelRequest) -> Result<ModelOutput, ModelError> {
        if let Some(result) = request.latest_tool_result() {
            let entries = result
                .output
                .get("entries")
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let entries = if entries.is_empty() {
                "(empty)"
            } else {
                &entries
            };
            return Ok(ModelOutput::FinalText(format!(
                "Workspace entries: {entries}"
            )));
        }

        Ok(ModelOutput::ToolCalls(vec![ToolCall::workspace_list(
            "deterministic-call-1",
            ".",
        )]))
    }
}

pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    fn request_body(&self, request: &ModelRequest) -> Result<Value, ModelError> {
        let messages = request
            .messages
            .iter()
            .map(openai_message)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(json!({
            "model": self.model,
            "messages": messages,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "workspace.list",
                    "description": "List direct entries beneath an allowed workspace directory.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"],
                        "additionalProperties": false
                    }
                }
            }],
            "tool_choice": "auto"
        }))
    }
}

#[async_trait]
impl ModelProvider for OpenAiCompatibleProvider {
    async fn next_turn(&self, request: ModelRequest) -> Result<ModelOutput, ModelError> {
        let mut builder = self
            .client
            .post(self.endpoint())
            .json(&self.request_body(&request)?);
        if !self.api_key.is_empty() {
            builder = builder.bearer_auth(&self.api_key);
        }

        let response = builder.send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(ModelError::HttpStatus { status, body });
        }

        let value: Value = serde_json::from_str(&body)?;
        parse_openai_chat_completion(value)
    }
}

fn openai_message(message: &ModelMessage) -> Result<Value, ModelError> {
    match message {
        ModelMessage::System(content) => Ok(json!({"role": "system", "content": content})),
        ModelMessage::User(content) => Ok(json!({"role": "user", "content": content})),
        ModelMessage::AssistantText(content) => {
            Ok(json!({"role": "assistant", "content": content}))
        }
        ModelMessage::AssistantToolCalls(calls) => {
            let tool_calls = calls
                .iter()
                .map(|call| {
                    Ok(json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.capability_id,
                            "arguments": serde_json::to_string(&call.arguments)?
                        }
                    }))
                })
                .collect::<Result<Vec<Value>, serde_json::Error>>()?;
            Ok(json!({
                "role": "assistant",
                "content": null,
                "tool_calls": tool_calls
            }))
        }
        ModelMessage::ToolResult(result) => Ok(json!({
            "role": "tool",
            "tool_call_id": result.call_id,
            "content": serde_json::to_string(&result.output)?
        })),
    }
}

pub fn parse_openai_chat_completion(value: Value) -> Result<ModelOutput, ModelError> {
    let message = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or(ModelError::InvalidResponse("missing choices[0].message"))?;

    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        if !tool_calls.is_empty() {
            let mut canonical = Vec::with_capacity(tool_calls.len());
            for raw_call in tool_calls {
                let id = required_str(raw_call, "id")?;
                let function = raw_call
                    .get("function")
                    .ok_or(ModelError::InvalidResponse("missing tool function"))?;
                let name = required_str(function, "name")?;
                let arguments = required_str(function, "arguments")?;
                let arguments: Value = serde_json::from_str(arguments)?;

                match name {
                    "workspace.list" => {
                        let path = arguments.get("path").and_then(Value::as_str).ok_or(
                            ModelError::InvalidResponse("workspace.list requires string path"),
                        )?;
                        canonical.push(ToolCall::workspace_list(id, path));
                    }
                    other => return Err(ModelError::UnsupportedTool(other.to_owned())),
                }
            }
            return Ok(ModelOutput::ToolCalls(canonical));
        }
    }

    let content =
        message
            .get("content")
            .and_then(Value::as_str)
            .ok_or(ModelError::InvalidResponse(
                "assistant response has neither tool calls nor text",
            ))?;
    Ok(ModelOutput::FinalText(content.to_owned()))
}

fn required_str<'a>(value: &'a Value, key: &'static str) -> Result<&'a str, ModelError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(ModelError::InvalidResponse(key))
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("model returned HTTP {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
    #[error("model JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid model response: {0}")]
    InvalidResponse(&'static str),
    #[error("unsupported model tool call: {0}")]
    UnsupportedTool(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_domain::ToolResult;
    use serde_json::json;

    #[tokio::test]
    async fn deterministic_provider_calls_workspace_then_finishes() {
        let provider = DeterministicProvider;
        let first = provider
            .next_turn(ModelRequest::user("List the workspace"))
            .await
            .expect("first deterministic turn");

        let calls = match first {
            ModelOutput::ToolCalls(calls) => calls,
            other => panic!("expected tool call, got {other:?}"),
        };
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].capability_id, "workspace.list");
        assert_eq!(calls[0].arguments, json!({"path": "."}));

        let second = provider
            .next_turn(ModelRequest::with_tool_result(
                "List the workspace",
                ToolResult {
                    call_id: calls[0].id.clone(),
                    capability_id: "workspace.list".into(),
                    output: json!({"entries": ["alpha.txt"]}),
                },
            ))
            .await
            .expect("second deterministic turn");

        assert_eq!(
            second,
            ModelOutput::FinalText("Workspace entries: alpha.txt".into())
        );
    }

    #[test]
    fn openai_codec_maps_workspace_tool_call_to_canonical_form() {
        let fixture = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "workspace.list",
                            "arguments": "{\"path\":\".\"}"
                        }
                    }]
                }
            }]
        });

        let output = parse_openai_chat_completion(fixture).expect("parse fixture");
        let calls = match output {
            ModelOutput::ToolCalls(calls) => calls,
            other => panic!("expected tool calls, got {other:?}"),
        };

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].capability_id, "workspace.list");
        assert_eq!(calls[0].arguments, json!({"path": "."}));
    }
}
