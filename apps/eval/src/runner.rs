//! Base and GAP flow runners.

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::Instant;

use gap::apply;
use gap::gap::{Artifact, Envelope};

use crate::client::{Message, OpenAIClient, StreamResult};
use crate::experiment::{clean_artifact, Experiment, TurnMetrics, TurnResult};

/// Everything recorded about one GAP edit turn, wherever in the
/// request → parse → apply pipeline it succeeded or failed.
struct TurnOutcome {
    parsed: bool,
    succeeded: bool,
    failure: Option<String>,
    env_name: String,
    latency_ms: u64,
    input_tokens: u64,
    output_tokens: u64,
    cached_input_tokens: u64,
    ttft_ms: Option<u64>,
    ttlt_ms: Option<u64>,
    median_itl_ms: Option<f64>,
    retried: Option<bool>,
}

impl TurnOutcome {
    /// Outcome for a stream that came back; parse/apply state not yet known.
    fn from_stream(r: &StreamResult, latency_ms: u64) -> Self {
        Self {
            parsed: false,
            succeeded: false,
            failure: None,
            env_name: String::new(),
            latency_ms,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cached_input_tokens: r.cached_input_tokens,
            ttft_ms: r.ttft_ms,
            ttlt_ms: r.ttlt_ms,
            median_itl_ms: r.median_itl_ms,
            retried: Some(r.retried),
        }
    }

    /// Outcome for a request that failed outright (no usage data available).
    fn request_failed(e: &anyhow::Error, latency_ms: u64) -> Self {
        Self {
            parsed: false,
            succeeded: false,
            failure: Some(format!("request failed: {e:#}")),
            env_name: String::new(),
            latency_ms,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            ttft_ms: None,
            ttlt_ms: None,
            median_itl_ms: None,
            retried: None,
        }
    }
}

/// JSON Schema for the LLM structured output (GAP envelope).
fn envelope_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["protocol", "id", "version", "name", "meta", "content"],
        "additionalProperties": false,
        "properties": {
            "protocol": {"type": "string"},
            "id": {"type": "string"},
            "version": {"type": "integer"},
            "name": {"type": "string", "enum": ["synthesize", "edit"]},
            "meta": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "format": {"type": ["string", "null"]},
                    "tokens_used": {"type": ["integer", "null"]},
                    "checksum": {"type": ["string", "null"]},
                    "state": {"type": ["string", "null"]}
                },
                "required": ["format", "tokens_used", "checksum", "state"]
            },
            // Edit content items match the protocol `EditOp` exactly:
            // {op, target, content}. The replacement text lives in `content`
            // (the apply engine ignores any other field). An earlier version of
            // this schema also exposed a spurious `body` field, which led models
            // to put replacement text there — apply then replaced targets with
            // empty strings, silently destroying artifacts. Do not reintroduce it.
            "content": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "op": {"type": "string", "enum": ["replace", "insert_before", "insert_after", "delete"]},
                        "target": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "type": {"type": "string", "enum": ["id", "pointer"]},
                                "value": {"type": "string"}
                            },
                            "required": ["type", "value"]
                        },
                        "content": {"type": ["string", "null"]}
                    },
                    "required": ["op", "target", "content"]
                }
            }
        }
    })
}

/// Run the base flow: full artifact regeneration each turn.
/// Returns (turn0_metrics, per_turn_results).
pub async fn run_base_flow(
    client: &OpenAIClient,
    system_prompt: &str,
    turn0_prompt: &str,
    edit_prompts: &[(String, String)],
    output_dir: &Path,
    ext: &str,
) -> Result<(TurnMetrics, Vec<TurnResult>)> {
    // Turn 0: generate initial artifact
    let mut messages = vec![
        Message {
            role: "system".into(),
            content: system_prompt.to_string(),
        },
        Message {
            role: "user".into(),
            content: turn0_prompt.to_string(),
        },
    ];

    let t0 = Instant::now();
    let result = client.chat_stream(&messages, None).await?;
    let latency_ms = t0.elapsed().as_millis() as u64;

    let artifact = clean_artifact(&result.text);
    fs::write(output_dir.join(format!("turn-0{ext}")), &artifact)?;

    // Add assistant response to conversation history
    messages.push(Message {
        role: "assistant".into(),
        content: result.text.clone(),
    });

    let turn0_metrics = TurnMetrics {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        cached_input_tokens: result.cached_input_tokens,
        latency_ms,
        artifact_bytes: artifact.len(),
        ttft_ms: result.ttft_ms,
        ttlt_ms: result.ttlt_ms,
        median_itl_ms: result.median_itl_ms,
    };

    // Edit turns: append to conversation, full regen each time
    let mut turn_results = Vec::new();
    for (turn_name, edit_prompt) in edit_prompts {
        let turn_num: usize = turn_name
            .strip_prefix("turn-")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        messages.push(Message {
            role: "user".into(),
            content: edit_prompt.clone(),
        });

        let t0 = Instant::now();
        let result = client.chat_stream(&messages, None).await?;
        let latency_ms = t0.elapsed().as_millis() as u64;

        let artifact = clean_artifact(&result.text);
        fs::write(output_dir.join(format!("{turn_name}{ext}")), &artifact)?;

        messages.push(Message {
            role: "assistant".into(),
            content: result.text.clone(),
        });

        turn_results.push(TurnResult {
            turn: turn_num,
            edit: edit_prompt.chars().take(80).collect(),
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
            cached_input_tokens: result.cached_input_tokens,
            latency_ms,
            output_bytes: artifact.len(),
            ttft_ms: result.ttft_ms,
            ttlt_ms: result.ttlt_ms,
            median_itl_ms: result.median_itl_ms,
            retried: Some(result.retried),
            failed: false,
            failure_reason: None,
            envelope_parsed: None,
            apply_succeeded: None,
            envelope_name: None,
        });
    }

    Ok((turn0_metrics, turn_results))
}

/// Run the stateless full-regen flow (spec Scenario B — the steelman baseline).
/// Each edit starts a fresh 2-message context that reads the *current* artifact
/// and regenerates the FULL artifact. This isolates GAP's output-token win
/// (C vs B) from the statelessness/input win (B vs A).
///
/// `seed` lets the caller share the base flow's plain turn-0 artifact (and its
/// creation metrics) so A and B operate on an identical document; when `None`,
/// turn-0 is generated fresh with `system_prompt`.
pub async fn run_stateless_flow(
    client: &OpenAIClient,
    system_prompt: &str,
    turn0_prompt: &str,
    edit_prompts: &[(String, String)],
    output_dir: &Path,
    ext: &str,
    seed: Option<(String, TurnMetrics)>,
) -> Result<(TurnMetrics, Vec<TurnResult>)> {
    // Turn 0: reuse the shared base artifact, or generate it fresh.
    let (mut artifact, turn0_metrics) = match seed {
        Some((art, t0)) => (art, t0),
        None => {
            let messages = vec![
                Message {
                    role: "system".into(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".into(),
                    content: turn0_prompt.to_string(),
                },
            ];
            let t0 = Instant::now();
            let result = client.chat_stream(&messages, None).await?;
            let latency_ms = t0.elapsed().as_millis() as u64;
            let artifact = clean_artifact(&result.text);
            let metrics = TurnMetrics {
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
                cached_input_tokens: result.cached_input_tokens,
                latency_ms,
                artifact_bytes: artifact.len(),
                ttft_ms: result.ttft_ms,
                ttlt_ms: result.ttlt_ms,
                median_itl_ms: result.median_itl_ms,
            };
            (artifact, metrics)
        }
    };
    fs::write(output_dir.join(format!("turn-0{ext}")), &artifact)?;

    // Edit turns: stateless, full regeneration each time.
    let mut turn_results = Vec::new();
    for (turn_name, edit_prompt) in edit_prompts {
        let turn_num: usize = turn_name
            .strip_prefix("turn-")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let user_msg = format!(
            "## Current Artifact\n\n```\n{artifact}\n```\n\n## Edit Instruction\n\n{edit_prompt}\n\nReturn the complete updated artifact, raw, with no commentary."
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".into(),
                content: user_msg,
            },
        ];

        let t0 = Instant::now();
        let result = client.chat_stream(&messages, None).await?;
        let latency_ms = t0.elapsed().as_millis() as u64;

        artifact = clean_artifact(&result.text);
        fs::write(output_dir.join(format!("{turn_name}{ext}")), &artifact)?;

        turn_results.push(TurnResult {
            turn: turn_num,
            edit: edit_prompt.chars().take(80).collect(),
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
            cached_input_tokens: result.cached_input_tokens,
            latency_ms,
            output_bytes: artifact.len(),
            ttft_ms: result.ttft_ms,
            ttlt_ms: result.ttlt_ms,
            median_itl_ms: result.median_itl_ms,
            retried: Some(result.retried),
            failed: false,
            failure_reason: None,
            envelope_parsed: None,
            apply_succeeded: None,
            envelope_name: None,
        });
    }

    Ok((turn0_metrics, turn_results))
}

/// Run the GAP flow: stateless per-turn envelope edits.
/// Returns (turn0_metrics, per_turn_results).
pub async fn run_gap_flow(
    client: &OpenAIClient,
    exp: &Experiment,
    output_dir: &Path,
) -> Result<(TurnMetrics, Vec<TurnResult>)> {
    let ext = &exp.ext;
    let schema = envelope_schema();

    // Turn 0: generate artifact with target markers
    let messages = vec![
        Message {
            role: "system".into(),
            content: exp.gap_init_system.clone(),
        },
        Message {
            role: "user".into(),
            content: exp.turn0_prompt.clone(),
        },
    ];

    let t0 = Instant::now();
    let result = client.chat_stream(&messages, None).await?;
    let latency_ms = t0.elapsed().as_millis() as u64;

    let mut artifact = clean_artifact(&result.text);
    fs::write(output_dir.join(format!("turn-0{ext}")), &artifact)?;

    let turn0_metrics = TurnMetrics {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        cached_input_tokens: result.cached_input_tokens,
        latency_ms,
        artifact_bytes: artifact.len(),
        ttft_ms: result.ttft_ms,
        ttlt_ms: result.ttlt_ms,
        median_itl_ms: result.median_itl_ms,
    };

    // Edit turns: stateless, envelope-based
    let mut turn_results = Vec::new();
    let mut version: u64 = 1;

    for (turn_name, edit_prompt) in &exp.edit_prompts {
        let turn_num: usize = turn_name
            .strip_prefix("turn-")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let user_msg = format!(
            "## Current Artifact\n\n```\n{artifact}\n```\n\n## Edit Instruction\n\n{edit_prompt}"
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: exp.gap_maintain_system.clone(),
            },
            Message {
                role: "user".into(),
                content: user_msg,
            },
        ];

        let t0 = Instant::now();
        let result = client.chat_stream(&messages, Some(&schema)).await;

        let outcome = match result {
            Ok(r) => {
                let mut o = TurnOutcome::from_stream(&r, t0.elapsed().as_millis() as u64);

                match serde_json::from_str::<Envelope>(&r.text) {
                    Ok(envelope) => {
                        o.parsed = true;
                        o.env_name = format!("{:?}", envelope.name).to_lowercase();

                        let art = Artifact {
                            id: envelope.id.clone(),
                            version: version.saturating_sub(1),
                            format: exp.format.clone(),
                            body: artifact.clone(),
                        };

                        match apply::apply(Some(&art), &envelope) {
                            Ok((new_art, _handle)) => {
                                fs::write(
                                    output_dir.join(format!("{turn_name}.json")),
                                    serde_json::to_string_pretty(&envelope)?,
                                )?;
                                artifact = new_art.body;
                                version += 1;
                                o.succeeded = true;
                            }
                            Err(e) => {
                                // Parsed but apply failed — keep the raw envelope
                                // for post-hoc analysis.
                                fs::write(output_dir.join(format!("{turn_name}.json")), &r.text)?;
                                o.failure = Some(format!("apply failed: {e:#}"));
                            }
                        }
                    }
                    Err(e) => {
                        o.failure = Some(format!("envelope parse failed: {e}"));
                    }
                }
                o
            }
            Err(e) => TurnOutcome::request_failed(&e, t0.elapsed().as_millis() as u64),
        };

        // Always write current artifact state
        fs::write(output_dir.join(format!("{turn_name}{ext}")), &artifact)?;

        turn_results.push(TurnResult {
            turn: turn_num,
            edit: edit_prompt.chars().take(80).collect(),
            input_tokens: outcome.input_tokens,
            output_tokens: outcome.output_tokens,
            cached_input_tokens: outcome.cached_input_tokens,
            latency_ms: outcome.latency_ms,
            output_bytes: artifact.len(),
            ttft_ms: outcome.ttft_ms,
            ttlt_ms: outcome.ttlt_ms,
            median_itl_ms: outcome.median_itl_ms,
            retried: outcome.retried,
            failed: !outcome.succeeded,
            failure_reason: outcome.failure,
            envelope_parsed: Some(outcome.parsed),
            apply_succeeded: Some(outcome.succeeded),
            envelope_name: Some(outcome.env_name),
        });
    }

    Ok((turn0_metrics, turn_results))
}
