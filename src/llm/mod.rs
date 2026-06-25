use std::io::{BufRead, BufReader};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::error::SrrResult;
use crate::types::{LlmMessage, LlmConfig};

/// Type alias for streaming LLM response iterator
pub type StreamIter = Box<dyn Iterator<Item = SrrResult<String>>>;

pub trait LlmClient: Send + Sync {
    fn chat(&self, messages: &[LlmMessage]) -> SrrResult<LlmResponse>;
    fn count_tokens(&self, text: &str) -> usize;
    fn set_model(&mut self, model: &str);

    fn chat_stream(&self, messages: &[LlmMessage]) -> SrrResult<StreamIter> {
        let response = self.chat(messages)?;
        Ok(Box::new(std::iter::once(Ok(response.content))))
    }

    fn embed(&self, texts: &[&str]) -> SrrResult<Vec<Vec<f32>>> {
        let _ = texts;
        Ok(vec![Vec::new(); texts.len()])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

// ── Shared SSE parsing ──

struct SseStream {
    reader: BufReader<reqwest::blocking::Response>,
    done: bool,
    parse_fn: fn(&str) -> Option<String>,
}

impl Iterator for SseStream {
    type Item = SrrResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    self.done = true;
                    return None;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with(":") {
                        continue;
                    }
                    if let Some(data) = trimmed.strip_prefix("data: ") {
                        if data == "[DONE]" || data.eq_ignore_ascii_case("[done]") {
                            self.done = true;
                            return None;
                        }
                        if let Some(text) = (self.parse_fn)(data) {
                            return Some(Ok(text));
                        }
                    }
                }
                Err(e) => {
                    self.done = true;
                    return Some(Err(e.into()));
                }
            }
        }
    }
}

fn parse_openai_sse(data: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<Value>(data) {
        val["choices"][0]["delta"]["content"].as_str().map(|s| s.to_string())
    } else {
        None
    }
}

fn parse_anthropic_sse(data: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<Value>(data) {
        if val["type"] == "content_block_delta" {
            val["delta"]["text"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

// ── Shared embedding helper ──

fn parse_openai_embedding(body: Value) -> SrrResult<Vec<Vec<f32>>> {
    let data = body["data"].as_array().ok_or_else(|| {
        crate::error::SrrError::Anyhow(anyhow::anyhow!("No 'data' in embedding response"))
    })?;
    let mut embeddings = Vec::with_capacity(data.len());
    for entry in data {
        let emb = entry["embedding"].as_array().ok_or_else(|| {
            crate::error::SrrError::Anyhow(anyhow::anyhow!("No 'embedding' in entry"))
        })?;
        let vec: Vec<f32> = emb.iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();
        embeddings.push(vec);
    }
    Ok(embeddings)
}

// ── OpenRouter Client ──

pub struct OpenRouterClient {
    config: LlmConfig,
    client: reqwest::blocking::Client,
}

impl OpenRouterClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn build_chat_headers(&self, request: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        request
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/user/srr")
            .header("X-Title", "SRR")
    }

    fn build_chat_payload(&self, messages: &[LlmMessage], stream: bool) -> Value {
        serde_json::json!({
            "model": self.config.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "stream": stream,
        })
    }

    fn parse_chat_response(&self, status: reqwest::StatusCode, body: Value) -> SrrResult<String> {
        if !status.is_success() {
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter API error ({}): {}", status.as_u16(), err_msg)));
        }
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(content)
    }
}

impl LlmClient for OpenRouterClient {
    fn set_model(&mut self, model: &str) {
        self.config.model = model.to_string();
    }

    fn chat(&self, messages: &[LlmMessage]) -> SrrResult<LlmResponse> {
        let payload = self.build_chat_payload(messages, false);
        let request = self.build_chat_headers(self.client.post("https://openrouter.ai/api/v1/chat/completions"))
            .json(&payload);
        let resp = request.send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter API request failed: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Failed to parse OpenRouter response: {e}")))?;
        let content = self.parse_chat_response(status, body)?;
        let input_tokens = 0;
        let output_tokens = content.len() / 4;
        Ok(LlmResponse { content, input_tokens, output_tokens })
    }

    fn count_tokens(&self, text: &str) -> usize {
        (text.len() as f64 / 4.0).ceil() as usize
    }

    fn chat_stream(&self, messages: &[LlmMessage]) -> SrrResult<StreamIter> {
        let payload = self.build_chat_payload(messages, true);
        let request = self.build_chat_headers(self.client.post("https://openrouter.ai/api/v1/chat/completions"))
            .json(&payload);
        let response = request.send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter streaming request failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            let body: Value = response.json().unwrap_or(Value::Null);
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str()).unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter stream error ({}): {}", status.as_u16(), err_msg)));
        }
        Ok(Box::new(SseStream {
            reader: BufReader::new(response),
            done: false,
            parse_fn: parse_openai_sse,
        }))
    }

    fn embed(&self, texts: &[&str]) -> SrrResult<Vec<Vec<f32>>> {
        let payload = serde_json::json!({
            "model": "openai/text-embedding-3-small",
            "input": texts,
        });
        let request = self.client.post("https://openrouter.ai/api/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/user/srr")
            .header("X-Title", "SRR")
            .json(&payload);
        let resp = request.send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter embedding request failed: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Failed to parse embedding response: {e}")))?;
        if !status.is_success() {
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str()).unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenRouter embedding error ({}): {}", status.as_u16(), err_msg)));
        }
        parse_openai_embedding(body)
    }
}

// ── OpenAI Client ──

pub struct OpenAiClient {
    config: LlmConfig,
    client: reqwest::blocking::Client,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl LlmClient for OpenAiClient {
    fn set_model(&mut self, model: &str) {
        self.config.model = model.to_string();
    }

    fn chat(&self, messages: &[LlmMessage]) -> SrrResult<LlmResponse> {
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI API request failed: {e}")))?;

        let status = resp.status();
        let body: Value = resp.json()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Failed to parse OpenAI response: {e}")))?;

        if !status.is_success() {
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI API error ({}): {}", status.as_u16(), err_msg)));
        }

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let usage = body.get("usage");
        let input_tokens = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as usize;
        let output_tokens = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as usize;

        Ok(LlmResponse { content, input_tokens, output_tokens })
    }

    fn count_tokens(&self, text: &str) -> usize {
        let s = text.len() as f64;
        (s / 4.0).ceil() as usize
    }

    fn chat_stream(&self, messages: &[LlmMessage]) -> SrrResult<StreamIter> {
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "stream": true,
        });
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI streaming request failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            let body: Value = response.json().unwrap_or(Value::Null);
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str()).unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI stream error ({}): {}", status.as_u16(), err_msg)));
        }
        Ok(Box::new(SseStream {
            reader: BufReader::new(response),
            done: false,
            parse_fn: parse_openai_sse,
        }))
    }

    fn embed(&self, texts: &[&str]) -> SrrResult<Vec<Vec<f32>>> {
        let payload = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": texts,
        });
        let resp = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI embedding request failed: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Failed to parse embedding response: {e}")))?;
        if !status.is_success() {
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str()).unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("OpenAI embedding error ({}): {}", status.as_u16(), err_msg)));
        }
        parse_openai_embedding(body)
    }
}

// ── Anthropic Client ──

pub struct AnthropicClient {
    config: LlmConfig,
    client: reqwest::blocking::Client,
}

impl AnthropicClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl LlmClient for AnthropicClient {
    fn set_model(&mut self, model: &str) {
        self.config.model = model.to_string();
    }

    fn chat(&self, messages: &[LlmMessage]) -> SrrResult<LlmResponse> {
        let system = messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let non_system: Vec<&LlmMessage> = messages.iter()
            .filter(|m| m.role != "system")
            .collect();

        let payload = serde_json::json!({
            "model": self.config.model,
            "system": system,
            "messages": non_system.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Anthropic API request failed: {e}")))?;

        let status = resp.status();
        let body: Value = resp.json()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Failed to parse Anthropic response: {e}")))?;

        if !status.is_success() {
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("Anthropic API error ({}): {}", status.as_u16(), err_msg)));
        }

        let content = body["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let usage = body.get("usage");
        let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as usize;
        let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as usize;

        Ok(LlmResponse { content, input_tokens, output_tokens })
    }

    fn count_tokens(&self, text: &str) -> usize {
        let s = text.len() as f64;
        (s / 3.5).ceil() as usize
    }

    fn chat_stream(&self, messages: &[LlmMessage]) -> SrrResult<StreamIter> {
        let system = messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let non_system: Vec<&LlmMessage> = messages.iter()
            .filter(|m| m.role != "system")
            .collect();

        let payload = serde_json::json!({
            "model": self.config.model,
            "system": system,
            "messages": non_system.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "stream": true,
        });
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| crate::error::SrrError::Anyhow(anyhow::anyhow!("Anthropic streaming request failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            let body: Value = response.json().unwrap_or(Value::Null);
            let err_msg = body.get("error").and_then(|e| e.get("message"))
                .and_then(|m| m.as_str()).unwrap_or("unknown error");
            return Err(crate::error::SrrError::Anyhow(anyhow::anyhow!("Anthropic stream error ({}): {}", status.as_u16(), err_msg)));
        }
        Ok(Box::new(SseStream {
            reader: BufReader::new(response),
            done: false,
            parse_fn: parse_anthropic_sse,
        }))
    }
}

// ── Dummy Client ──

pub struct DummyClient;

impl LlmClient for DummyClient {
    fn set_model(&mut self, _model: &str) {}

    fn chat(&self, messages: &[LlmMessage]) -> SrrResult<LlmResponse> {
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        let content = format!("[Dummy response to: {}...]", &last[..last.len().min(80)]);
        Ok(LlmResponse { content, input_tokens: 0, output_tokens: 0 })
    }

    fn count_tokens(&self, text: &str) -> usize {
        let s = text.len() as f64;
        (s / 4.0).ceil() as usize
    }

    fn embed(&self, texts: &[&str]) -> SrrResult<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0f32; 4]).collect())
    }
}
