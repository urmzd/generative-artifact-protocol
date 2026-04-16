//! OpenAI-compatible streaming chat client.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    api_base: String,
    api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub struct StreamResult {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub ttft_ms: Option<u64>,
    pub ttlt_ms: Option<u64>,
    pub median_itl_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    choices: Option<Vec<ChunkChoice>>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
}

impl OpenAIClient {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
            model,
        }
    }

    /// Stream a chat completion. Returns the full text and timing metrics.
    pub async fn chat_stream(
        &self,
        messages: &[Message],
        json_schema: Option<&serde_json::Value>,
    ) -> Result<StreamResult> {
        let url = format!("{}/chat/completions", self.api_base);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "stream_options": {"include_usage": true},
        });

        if let Some(schema) = json_schema {
            body["response_format"] = serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "gap_envelope",
                    "strict": true,
                    "schema": schema,
                }
            });
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("API error {status}: {body}"));
        }

        let t0 = Instant::now();
        let mut chunks: Vec<String> = Vec::new();
        let mut timestamps: Vec<Instant> = Vec::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                let data = line.strip_prefix("data: ").unwrap_or(&line);

                if let Ok(chunk) = serde_json::from_str::<ChatChunk>(data) {
                    if let Some(usage) = &chunk.usage {
                        input_tokens = usage.prompt_tokens.unwrap_or(0);
                        output_tokens = usage.completion_tokens.unwrap_or(0);
                    }

                    if let Some(choices) = &chunk.choices {
                        for choice in choices {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    if !content.is_empty() {
                                        timestamps.push(Instant::now());
                                        chunks.push(content.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let text = chunks.concat();

        let ttft_ms = timestamps.first().map(|t| t.duration_since(t0).as_millis() as u64);
        let ttlt_ms = timestamps.last().map(|t| t.duration_since(t0).as_millis() as u64);
        let median_itl_ms = if timestamps.len() > 1 {
            let mut intervals: Vec<f64> = timestamps
                .windows(2)
                .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.0)
                .collect();
            intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = intervals.len() / 2;
            Some(if intervals.len() % 2 == 0 {
                (intervals[mid - 1] + intervals[mid]) / 2.0
            } else {
                intervals[mid]
            })
        } else {
            None
        };

        Ok(StreamResult {
            text,
            input_tokens,
            output_tokens,
            ttft_ms,
            ttlt_ms,
            median_itl_ms,
        })
    }
}
