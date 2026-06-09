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
    /// Sampling temperature. `Some(0.0)` maximizes reproducibility (see
    /// EXPERIMENT.md); `None` omits the field for models that reject it.
    pub temperature: Option<f32>,
    /// Deterministic sampling seed where the provider supports it.
    pub seed: Option<u64>,
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
    /// Portion of `input_tokens` served from the provider's prompt cache
    /// (billed at a discount). Lets us model cost with caching on vs off.
    pub cached_input_tokens: u64,
    pub ttft_ms: Option<u64>,
    pub ttlt_ms: Option<u64>,
    pub median_itl_ms: Option<f64>,
    /// True when the request only succeeded after at least one retry. Retried
    /// turns carry backoff sleep in their wall-clock and must be excluded from
    /// latency aggregates.
    pub retried: bool,
}

/// Pop complete `\n`-terminated lines off `buf`, leaving any trailing partial
/// line in place. Splitting at line boundaries (not chunk boundaries) keeps
/// multi-byte UTF-8 sequences that straddle network chunks intact.
fn drain_lines(buf: &mut Vec<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        lines.push(String::from_utf8_lossy(&line).trim().to_string());
    }
    lines
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
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct PromptTokensDetails {
    cached_tokens: Option<u64>,
}

impl OpenAIClient {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        // Reasoning-class models (o-series, gpt-5 family) reject `temperature`
        // and only accept the default (1.0). Omit it for those; pin it to 0 for
        // everything else so chat models are reproducible (see EXPERIMENT.md).
        let m = model.to_lowercase();
        let is_reasoning = m.starts_with("o1")
            || m.starts_with("o3")
            || m.starts_with("o4")
            || m.starts_with("gpt-5");
        let temperature = if is_reasoning { None } else { Some(0.0) };
        Self {
            client: Client::new(),
            api_base,
            api_key,
            model,
            temperature,
            seed: Some(42),
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

        if let Some(temp) = self.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(seed) = self.seed {
            body["seed"] = serde_json::json!(seed);
        }

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

        // Retry transient failures (429 rate limits, 5xx) with exponential
        // backoff. Low-tier keys return spurious 429 `insufficient_quota` under
        // burst; without this a single blip aborts a multi-hour suite run.
        const MAX_ATTEMPTS: u32 = 6;
        let mut attempt: u32 = 0;
        let (resp, t0, retried) = loop {
            // TTFT is anchored here, immediately before the request goes out,
            // so it includes connection setup, upload, queueing, and prefill.
            let t0 = Instant::now();
            let send_result = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            let retryable = match &send_result {
                Ok(r) => {
                    let s = r.status();
                    s == reqwest::StatusCode::TOO_MANY_REQUESTS || s.is_server_error()
                }
                Err(_) => true, // connection/timeout errors are retryable
            };

            attempt += 1;
            if !retryable || attempt >= MAX_ATTEMPTS {
                break (send_result?, t0, attempt > 1);
            }

            // Honor Retry-After when present, else exponential backoff: 1,2,4,8,16s.
            let retry_after = send_result
                .as_ref()
                .ok()
                .and_then(|r| r.headers().get(reqwest::header::RETRY_AFTER))
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            let backoff_secs = retry_after.unwrap_or_else(|| 1u64 << (attempt - 1).min(4));
            eprintln!(
                "    transient API failure (attempt {attempt}/{MAX_ATTEMPTS}), retrying in {backoff_secs}s"
            );
            tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("API error {status}: {body}"));
        }
        let mut chunks: Vec<String> = Vec::new();
        let mut timestamps: Vec<Instant> = Vec::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut cached_input_tokens: u64 = 0;

        let mut stream = resp.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buffer.extend_from_slice(&bytes);

            for line in drain_lines(&mut buffer) {
                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                let data = line.strip_prefix("data: ").unwrap_or(&line);

                if let Ok(chunk) = serde_json::from_str::<ChatChunk>(data) {
                    if let Some(usage) = &chunk.usage {
                        input_tokens = usage.prompt_tokens.unwrap_or(0);
                        output_tokens = usage.completion_tokens.unwrap_or(0);
                        cached_input_tokens = usage
                            .prompt_tokens_details
                            .as_ref()
                            .and_then(|d| d.cached_tokens)
                            .unwrap_or(0);
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

        // Some OpenAI-compatible providers buffer the whole completion and
        // emit it as one or two giant chunks (emulated streaming). The first
        // chunk then arrives near the end, so TTFT and ITL are meaningless;
        // null them rather than publish them. TTLT (when the last token
        // reached the client) is still real. Threshold: real token streaming
        // never averages >100 tokens per content delta.
        let emulated = output_tokens > 100 && (timestamps.len() as u64) < output_tokens / 100;

        let ttft_ms = if emulated {
            None
        } else {
            timestamps
                .first()
                .map(|t| t.duration_since(t0).as_millis() as u64)
        };
        let ttlt_ms = timestamps
            .last()
            .map(|t| t.duration_since(t0).as_millis() as u64);
        let median_itl_ms = if emulated {
            None
        } else if timestamps.len() > 1 {
            let mut intervals: Vec<f64> = timestamps
                .windows(2)
                .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.0)
                .collect();
            intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = intervals.len() / 2;
            Some(if intervals.len().is_multiple_of(2) {
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
            cached_input_tokens,
            ttft_ms,
            ttlt_ms,
            median_itl_ms,
            retried,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn drain_lines_reassembles_multibyte_utf8_split_across_chunks() {
        let payload = "data: {\"x\":\"héllo wörld ✓\"}\n";
        let bytes = payload.as_bytes();
        // Split mid-'é' (a two-byte sequence): the old per-chunk lossy decode
        // turned both halves into U+FFFD.
        let split = payload.find('é').unwrap() + 1;
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&bytes[..split]);
        assert!(drain_lines(&mut buf).is_empty());
        buf.extend_from_slice(&bytes[split..]);
        let lines = drain_lines(&mut buf);
        assert_eq!(lines, vec![payload.trim().to_string()]);
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_lines_keeps_trailing_partial_line() {
        let mut buf = b"data: a\ndata: b".to_vec();
        assert_eq!(drain_lines(&mut buf), vec!["data: a".to_string()]);
        assert_eq!(buf, b"data: b".to_vec());
    }

    const SSE_BODY: &str = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"hé\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n\n",
        "data: [DONE]\n\n",
    );

    fn test_messages() -> Vec<Message> {
        vec![Message {
            role: "user".into(),
            content: "x".into(),
        }]
    }

    #[tokio::test]
    async fn chat_stream_parses_sse_and_reports_no_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(SSE_BODY, "text/event-stream"))
            .mount(&server)
            .await;

        let client = OpenAIClient::new(server.uri(), "k".into(), "test-model".into());
        let res = client.chat_stream(&test_messages(), None).await.unwrap();
        assert_eq!(res.text, "héllo");
        assert_eq!(res.input_tokens, 7);
        assert_eq!(res.output_tokens, 2);
        assert!(!res.retried);
        assert!(res.ttft_ms.is_some());
        assert!(res.ttlt_ms.is_some());
    }

    #[tokio::test]
    async fn chat_stream_flags_retried_after_429() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(SSE_BODY, "text/event-stream"))
            .expect(1)
            .mount(&server)
            .await;

        let client = OpenAIClient::new(server.uri(), "k".into(), "test-model".into());
        let res = client.chat_stream(&test_messages(), None).await.unwrap();
        assert_eq!(res.text, "héllo");
        assert!(res.retried);
        // TTFT is anchored at the successful attempt's send, not at the start
        // of the retry loop: it must not contain the 1 s backoff sleep.
        assert!(res.ttft_ms.unwrap() < 1000);
    }
}
