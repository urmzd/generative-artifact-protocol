//! Base and GAP flow runners.

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::Instant;

use gap::apply;
use gap::gap::{Artifact, Envelope};

use crate::client::{Message, OpenAIClient};
use crate::experiment::{clean_artifact, TurnMetrics, TurnResult};

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
            "content": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "body": {"type": ["string", "null"]},
                        "op": {"type": ["string", "null"], "enum": ["replace", "insert_before", "insert_after", "delete", null]},
                        "target": {
                            "anyOf": [
                                {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "type": {"type": "string", "enum": ["id", "pointer"]},
                                        "value": {"type": "string"}
                                    },
                                    "required": ["type", "value"]
                                },
                                {"type": "null"}
                            ]
                        },
                        "content": {"type": ["string", "null"]}
                    },
                    "required": ["body", "op", "target", "content"]
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
        Message { role: "system".into(), content: system_prompt.to_string() },
        Message { role: "user".into(), content: turn0_prompt.to_string() },
    ];

    let t0 = Instant::now();
    let result = client.chat_stream(&messages, None).await?;
    let latency_ms = t0.elapsed().as_millis() as u64;

    let artifact = clean_artifact(&result.text);
    fs::write(output_dir.join(format!("turn-0{ext}")), &artifact)?;

    // Add assistant response to conversation history
    messages.push(Message { role: "assistant".into(), content: result.text.clone() });

    let turn0_metrics = TurnMetrics {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
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

        messages.push(Message { role: "user".into(), content: edit_prompt.clone() });

        let t0 = Instant::now();
        let result = client.chat_stream(&messages, None).await?;
        let latency_ms = t0.elapsed().as_millis() as u64;

        let artifact = clean_artifact(&result.text);
        fs::write(output_dir.join(format!("{turn_name}{ext}")), &artifact)?;

        messages.push(Message { role: "assistant".into(), content: result.text.clone() });

        turn_results.push(TurnResult {
            turn: turn_num,
            edit: edit_prompt.chars().take(80).collect(),
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
            latency_ms,
            output_bytes: artifact.len(),
            ttft_ms: result.ttft_ms,
            ttlt_ms: result.ttlt_ms,
            median_itl_ms: result.median_itl_ms,
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
    init_system: &str,
    maintain_system: &str,
    turn0_prompt: &str,
    edit_prompts: &[(String, String)],
    format: &str,
    output_dir: &Path,
    ext: &str,
) -> Result<(TurnMetrics, Vec<TurnResult>)> {
    let schema = envelope_schema();

    // Turn 0: generate artifact with target markers
    let messages = vec![
        Message { role: "system".into(), content: init_system.to_string() },
        Message { role: "user".into(), content: turn0_prompt.to_string() },
    ];

    let t0 = Instant::now();
    let result = client.chat_stream(&messages, None).await?;
    let latency_ms = t0.elapsed().as_millis() as u64;

    let mut artifact = clean_artifact(&result.text);
    fs::write(output_dir.join(format!("turn-0{ext}")), &artifact)?;

    let turn0_metrics = TurnMetrics {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        latency_ms,
        artifact_bytes: artifact.len(),
        ttft_ms: result.ttft_ms,
        ttlt_ms: result.ttlt_ms,
        median_itl_ms: result.median_itl_ms,
    };

    // Edit turns: stateless, envelope-based
    let mut turn_results = Vec::new();
    let mut version: u64 = 1;

    for (turn_name, edit_prompt) in edit_prompts {
        let turn_num: usize = turn_name
            .strip_prefix("turn-")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let user_msg = format!(
            "## Current Artifact\n\n```\n{artifact}\n```\n\n## Edit Instruction\n\n{edit_prompt}"
        );

        let messages = vec![
            Message { role: "system".into(), content: maintain_system.to_string() },
            Message { role: "user".into(), content: user_msg },
        ];

        let t0 = Instant::now();
        let result = client.chat_stream(&messages, Some(&schema)).await;

        let (parsed, succeeded, env_name, latency_ms, input_tokens, output_tokens, ttft_ms, ttlt_ms, median_itl_ms) =
            match result {
                Ok(r) => {
                    let latency_ms = t0.elapsed().as_millis() as u64;

                    // Try to parse the envelope
                    match serde_json::from_str::<Envelope>(&r.text) {
                        Ok(envelope) => {
                            let env_name = format!("{:?}", envelope.name).to_lowercase();

                            // Try to apply
                            let art = Artifact {
                                id: envelope.id.clone(),
                                version: version.saturating_sub(1),
                                format: format.to_string(),
                                body: artifact.clone(),
                            };

                            match apply::apply(Some(&art), &envelope) {
                                Ok((new_art, _handle)) => {
                                    // Write envelope JSON
                                    fs::write(
                                        output_dir.join(format!("{turn_name}.json")),
                                        serde_json::to_string_pretty(&envelope)?,
                                    )?;
                                    artifact = new_art.body;
                                    version += 1;
                                    (true, true, env_name, latency_ms, r.input_tokens, r.output_tokens, r.ttft_ms, r.ttlt_ms, r.median_itl_ms)
                                }
                                Err(_) => {
                                    // Parsed but apply failed
                                    fs::write(
                                        output_dir.join(format!("{turn_name}.json")),
                                        &r.text,
                                    )?;
                                    (true, false, env_name, latency_ms, r.input_tokens, r.output_tokens, r.ttft_ms, r.ttlt_ms, r.median_itl_ms)
                                }
                            }
                        }
                        Err(_) => {
                            // Parse failed
                            (false, false, String::new(), latency_ms, r.input_tokens, r.output_tokens, r.ttft_ms, r.ttlt_ms, r.median_itl_ms)
                        }
                    }
                }
                Err(_) => {
                    let latency_ms = t0.elapsed().as_millis() as u64;
                    (false, false, String::new(), latency_ms, 0, 0, None, None, None)
                }
            };

        // Always write current artifact state
        fs::write(output_dir.join(format!("{turn_name}{ext}")), &artifact)?;

        turn_results.push(TurnResult {
            turn: turn_num,
            edit: edit_prompt.chars().take(80).collect(),
            input_tokens,
            output_tokens,
            latency_ms,
            output_bytes: artifact.len(),
            ttft_ms,
            ttlt_ms,
            median_itl_ms,
            failed: !succeeded,
            failure_reason: if succeeded { None } else { Some("parse or apply failed".into()) },
            envelope_parsed: Some(parsed),
            apply_succeeded: Some(succeeded),
            envelope_name: Some(env_name),
        });
    }

    Ok((turn0_metrics, turn_results))
}
