//! Experiment loading and orchestration.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::client::OpenAIClient;
use crate::runner;
use crate::scorer;

pub struct RunConfig {
    pub experiments_dir: PathBuf,
    pub count: usize,
    pub id_filter: Option<String>,
    pub flow: String,
    pub model: String,
    pub api_base: String,
    pub api_key: String,
    pub skip_eval: bool,
}

#[derive(Debug)]
pub struct Experiment {
    pub id: String,
    pub dir: PathBuf,
    pub format: String,
    pub ext: String,
    pub base_system: String,
    pub gap_init_system: String,
    pub gap_maintain_system: String,
    pub turn0_prompt: String,
    pub edit_prompts: Vec<(String, String)>,
}

/// Metrics JSON written per experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub experiment_id: String,
    pub model: String,
    pub provider: String,
    pub timestamp: String,
    pub format: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_turn0: Option<TurnMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_turn0: Option<TurnMetrics>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_flow: Option<FlowMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_flow: Option<GapFlowMetrics>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<Comparison>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_table: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub artifact_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttlt_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_itl_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    pub turn: usize,
    pub edit: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub output_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttlt_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_itl_ms: Option<f64>,
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    // GAP-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope_parsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_succeeded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMetrics {
    pub per_turn: Vec<TurnResult>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapFlowMetrics {
    #[serde(flatten)]
    pub flow: FlowMetrics,
    pub envelope_parse_rate: f64,
    pub apply_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub output_token_savings_pct: f64,
    pub input_token_savings_pct: f64,
    pub latency_savings_pct: f64,
}

/// Map MIME type → file extension.
pub fn format_to_ext(fmt: &str) -> &'static str {
    match fmt {
        "text/html" => ".html",
        "text/x-python" => ".py",
        "application/javascript" => ".js",
        "text/typescript" => ".ts",
        "application/json" => ".json",
        "text/x-yaml" => ".yaml",
        "text/x-toml" => ".toml",
        "text/x-rust" => ".rs",
        "text/x-go" => ".go",
        "text/css" => ".css",
        "text/x-shellscript" => ".sh",
        "text/markdown" => ".md",
        "image/svg+xml" => ".svg",
        "application/xml" => ".xml",
        "text/x-java" => ".java",
        "text/x-ruby" => ".rb",
        "application/sql" => ".sql",
        _ => ".txt",
    }
}

/// Parse format from README.md `**Format:** text/html` line.
fn parse_format(readme: &str) -> String {
    for line in readme.lines() {
        if let Some(rest) = line.strip_prefix("**Format:**") {
            let fmt = rest.split('|').next().unwrap_or(rest).trim();
            return fmt.to_string();
        }
    }
    "text/html".to_string()
}

/// Load a single experiment from its directory.
fn load_experiment(dir: &Path, spec_init: &str, spec_maintain: &str) -> Result<Experiment> {
    let id = dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let readme = fs::read_to_string(dir.join("README.md"))
        .unwrap_or_default();
    let format = parse_format(&readme);
    let ext = format_to_ext(&format).to_string();

    let base_input = dir.join("inputs/base");
    let gap_input = dir.join("inputs/gap");

    let base_system = fs::read_to_string(base_input.join("system.md"))
        .unwrap_or_default();

    let gap_init_system = fs::read_to_string(gap_input.join("init-system.md"))
        .unwrap_or_else(|_| format!("{base_system}\n\n{spec_init}"));

    let gap_maintain_system = fs::read_to_string(gap_input.join("maintain-system.md"))
        .unwrap_or_else(|_| format!("{base_system}\n\n{spec_maintain}"));

    let turn0_prompt = fs::read_to_string(base_input.join("turn-0.md"))
        .context("missing turn-0.md")?;

    let mut edit_prompts = Vec::new();
    for i in 1.. {
        let path = base_input.join(format!("turn-{i}.md"));
        match fs::read_to_string(&path) {
            Ok(content) => edit_prompts.push((format!("turn-{i}"), content)),
            Err(_) => break,
        }
    }

    Ok(Experiment {
        id,
        dir: dir.to_path_buf(),
        format,
        ext,
        base_system,
        gap_init_system,
        gap_maintain_system,
        turn0_prompt,
        edit_prompts,
    })
}

/// Discover and load all experiments.
fn load_experiments(dir: &Path, spec_init: &str, spec_maintain: &str) -> Result<Vec<Experiment>> {
    let mut experiments = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir() && e.path().join("README.md").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        match load_experiment(&entry.path(), spec_init, spec_maintain) {
            Ok(exp) => experiments.push(exp),
            Err(e) => eprintln!("skip {}: {e}", entry.file_name().to_string_lossy()),
        }
    }
    Ok(experiments)
}

/// Clean raw LLM output — strip markdown fences and <think> blocks.
pub fn clean_artifact(text: &str) -> String {
    let mut s = text.trim().to_string();
    if s.starts_with("```") {
        if let Some(nl) = s.find('\n') {
            s = s[nl + 1..].to_string();
        }
    }
    if s.ends_with("```") {
        s = s[..s.len() - 3].trim_end().to_string();
    }
    let re = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap();
    re.replace_all(&s, "").trim().to_string()
}

/// Run all experiments.
pub async fn run_all(config: &RunConfig) -> Result<()> {
    let spec_dir = config.experiments_dir.parent().unwrap_or(Path::new("."));
    let spec_init = fs::read_to_string(spec_dir.join("gap-spec-init.md")).unwrap_or_default();
    let spec_maintain = fs::read_to_string(spec_dir.join("gap-spec-maintain.md")).unwrap_or_default();

    let mut experiments = load_experiments(&config.experiments_dir, &spec_init, &spec_maintain)?;

    if let Some(ref prefix) = config.id_filter {
        experiments.retain(|e| e.id.starts_with(prefix));
    }
    if config.count > 0 && experiments.len() > config.count {
        experiments.truncate(config.count);
    }

    let client = OpenAIClient::new(
        config.api_base.clone(),
        config.api_key.clone(),
        config.model.clone(),
    );

    let total = experiments.len();
    for (i, exp) in experiments.iter().enumerate() {
        let metrics_path = exp.dir.join("metrics.json");
        if metrics_path.exists() {
            eprintln!("[{}/{}] skip {} (metrics.json exists)", i + 1, total, exp.id);
            continue;
        }

        eprintln!("[{}/{}] running {}", i + 1, total, exp.id);

        let metrics = run_single(&client, exp, &config.flow).await?;

        let json = serde_json::to_string_pretty(&metrics)?;
        fs::write(&metrics_path, &json)?;
        eprintln!("  → wrote {}", metrics_path.display());

        if !config.skip_eval {
            if let Err(e) = scorer::score_experiment(&exp.dir) {
                eprintln!("  → scoring failed: {e}");
            }
        }
    }

    Ok(())
}

async fn run_single(client: &OpenAIClient, exp: &Experiment, flow: &str) -> Result<Metrics> {
    let run_base = flow == "both" || flow == "base";
    let run_gap = flow == "both" || flow == "gap";

    let base_out = exp.dir.join("outputs/base");
    let gap_out = exp.dir.join("outputs/gap");
    fs::create_dir_all(&base_out)?;
    fs::create_dir_all(&gap_out)?;

    let mut metrics = Metrics {
        experiment_id: exp.id.clone(),
        model: client.model.clone(),
        provider: "openai".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        format: exp.format.clone(),
        base_turn0: None,
        gap_turn0: None,
        default_flow: None,
        gap_flow: None,
        comparison: None,
        token_table: None,
        quality: None,
    };

    // Base flow
    if run_base {
        eprintln!("  base flow...");
        let (base_t0, base_turns) = runner::run_base_flow(
            client, &exp.base_system, &exp.turn0_prompt, &exp.edit_prompts,
            &base_out, &exp.ext,
        ).await?;
        let base_flow = to_flow_metrics(&base_turns);
        metrics.base_turn0 = Some(base_t0);
        metrics.default_flow = Some(base_flow);
    }

    // GAP flow
    if run_gap {
        eprintln!("  gap flow...");
        let (gap_t0, gap_turns) = runner::run_gap_flow(
            client, &exp.gap_init_system, &exp.gap_maintain_system,
            &exp.turn0_prompt, &exp.edit_prompts,
            &exp.format, &gap_out, &exp.ext,
        ).await?;

        let total_turns = gap_turns.len() as f64;
        let parsed = gap_turns.iter().filter(|t| t.envelope_parsed == Some(true)).count() as f64;
        let applied = gap_turns.iter().filter(|t| t.apply_succeeded == Some(true)).count() as f64;

        let gap_flow = GapFlowMetrics {
            flow: to_flow_metrics(&gap_turns),
            envelope_parse_rate: if total_turns > 0.0 { parsed / total_turns } else { 0.0 },
            apply_success_rate: if total_turns > 0.0 { applied / total_turns } else { 0.0 },
        };
        metrics.gap_turn0 = Some(gap_t0);
        metrics.gap_flow = Some(gap_flow);
    }

    // Comparison
    if let (Some(ref base), Some(ref gap)) = (&metrics.default_flow, &metrics.gap_flow) {
        let base_out_tokens = base.total_output_tokens as f64;
        let gap_out_tokens = gap.flow.total_output_tokens as f64;
        let base_in_tokens = base.total_input_tokens as f64;
        let gap_in_tokens = gap.flow.total_input_tokens as f64;
        let base_latency = base.total_latency_ms as f64;
        let gap_latency = gap.flow.total_latency_ms as f64;

        metrics.comparison = Some(Comparison {
            output_token_savings_pct: if base_out_tokens > 0.0 {
                round1((1.0 - gap_out_tokens / base_out_tokens) * 100.0)
            } else { 0.0 },
            input_token_savings_pct: if base_in_tokens > 0.0 {
                round1((1.0 - gap_in_tokens / base_in_tokens) * 100.0)
            } else { 0.0 },
            latency_savings_pct: if base_latency > 0.0 {
                round1((1.0 - gap_latency / base_latency) * 100.0)
            } else { 0.0 },
        });
    }

    Ok(metrics)
}

fn to_flow_metrics(turns: &[TurnResult]) -> FlowMetrics {
    FlowMetrics {
        total_input_tokens: turns.iter().map(|t| t.input_tokens).sum(),
        total_output_tokens: turns.iter().map(|t| t.output_tokens).sum(),
        total_latency_ms: turns.iter().map(|t| t.latency_ms).sum(),
        per_turn: turns.to_vec(),
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
