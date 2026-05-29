//! Assertion-oracle correctness checks against GAP eval artifacts.
//!
//! Each turn may carry a `checks/turn-N.json` file describing a set of
//! deterministic assertions ("oracles") to run against the produced artifact.
//! Results are written into `metrics.json` under a `correctness` object for
//! both the GAP and BASE flows so they can be compared.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::experiment::format_to_ext;

/// A single assertion oracle. Tagged by `kind` in JSON.
///
/// ```json
/// { "kind": "contains", "value": "<h1>", "desc": "has a heading" }
/// { "kind": "json_pointer_equals", "pointer": "/name", "value": "widget" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// Artifact parses as valid JSON.
    ValidJson {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
    /// Artifact contains the literal substring `value`.
    Contains {
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
    /// Artifact does NOT contain the literal substring `value`.
    Absent {
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
    /// `pattern` matches exactly `expected` non-overlapping times.
    RegexCount {
        pattern: String,
        expected: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
    /// `pattern` matches at least `min` non-overlapping times.
    RegexCountAtLeast {
        pattern: String,
        min: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
    /// Artifact parses as JSON and the value at `pointer` equals `value`.
    JsonPointerEquals {
        pointer: String,
        value: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
    },
}

impl Check {
    /// Short, stable kind discriminator used in result rows.
    pub fn kind(&self) -> &'static str {
        match self {
            Check::ValidJson { .. } => "valid_json",
            Check::Contains { .. } => "contains",
            Check::Absent { .. } => "absent",
            Check::RegexCount { .. } => "regex_count",
            Check::RegexCountAtLeast { .. } => "regex_count_at_least",
            Check::JsonPointerEquals { .. } => "json_pointer_equals",
        }
    }

    /// Optional human-authored description.
    pub fn desc(&self) -> Option<&str> {
        match self {
            Check::ValidJson { desc }
            | Check::Contains { desc, .. }
            | Check::Absent { desc, .. }
            | Check::RegexCount { desc, .. }
            | Check::RegexCountAtLeast { desc, .. }
            | Check::JsonPointerEquals { desc, .. } => desc.as_deref(),
        }
    }
}

/// The set of checks to run for a single turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnChecks {
    pub turn: usize,
    pub checks: Vec<Check>,
}

/// Result row for a single evaluated check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub kind: String,
    pub pass: bool,
    pub reason: String,
}

/// Per-turn correctness result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCorrectness {
    pub turn: usize,
    pub passed: usize,
    pub total: usize,
    pub checks: Vec<CheckResult>,
}

/// Correctness summary written into `metrics.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correctness {
    pub per_turn: Vec<TurnCorrectness>,
    pub pass_rate: f64,
    pub base_pass_rate: f64,
}

/// Evaluate a single check against an artifact, returning `(pass, reason)`.
pub fn evaluate_check(check: &Check, artifact: &str) -> (bool, String) {
    match check {
        Check::ValidJson { .. } => match serde_json::from_str::<serde_json::Value>(artifact) {
            Ok(_) => (true, "artifact is valid JSON".to_string()),
            Err(e) => (false, format!("not valid JSON: {e}")),
        },

        Check::Contains { value, .. } => {
            if artifact.contains(value.as_str()) {
                (true, format!("found {value:?}"))
            } else {
                (false, format!("missing {value:?}"))
            }
        }

        Check::Absent { value, .. } => {
            if artifact.contains(value.as_str()) {
                (false, format!("found {value:?} but expected absent"))
            } else {
                (true, format!("{value:?} is absent"))
            }
        }

        Check::RegexCount { pattern, expected, .. } => {
            let re = match Regex::new(pattern) {
                Ok(re) => re,
                Err(e) => return (false, format!("invalid regex {pattern:?}: {e}")),
            };
            let count = re.find_iter(artifact).count();
            if count == *expected {
                (true, format!("{pattern:?} matched {count} time(s)"))
            } else {
                (false, format!("{pattern:?} matched {count} time(s), expected {expected}"))
            }
        }

        Check::RegexCountAtLeast { pattern, min, .. } => {
            let re = match Regex::new(pattern) {
                Ok(re) => re,
                Err(e) => return (false, format!("invalid regex {pattern:?}: {e}")),
            };
            let count = re.find_iter(artifact).count();
            if count >= *min {
                (true, format!("{pattern:?} matched {count} time(s) (>= {min})"))
            } else {
                (false, format!("{pattern:?} matched {count} time(s), expected >= {min}"))
            }
        }

        Check::JsonPointerEquals { pointer, value, .. } => {
            let parsed: serde_json::Value = match serde_json::from_str(artifact) {
                Ok(v) => v,
                Err(e) => return (false, format!("not valid JSON: {e}")),
            };
            match parsed.pointer(pointer) {
                Some(actual) if actual == value => {
                    (true, format!("{pointer} == {value}"))
                }
                Some(actual) => (false, format!("{pointer} == {actual}, expected {value}")),
                None => (false, format!("{pointer} not present")),
            }
        }
    }
}

/// Strip `<gap:target ...>` and `</gap:target>` markers.
///
/// Kept local to mirror `scorer.rs` (the function there is private).
fn strip_gap_markers(text: &str) -> String {
    let re = Regex::new(r"</?gap:target[^>]*>").unwrap();
    re.replace_all(text, "").to_string()
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// Read an artifact for a turn from `outputs/<flow>/turn-N.<ext>`.
///
/// GAP markers are stripped for every format EXCEPT JSON, where the markers
/// would never appear inside a valid JSON document and stripping is a no-op
/// that could otherwise corrupt JSON string contents.
fn read_artifact(flow_dir: &Path, turn: usize, ext: &str) -> Option<String> {
    let path = flow_dir.join(format!("turn-{turn}{ext}"));
    let raw = fs::read_to_string(&path).ok()?;
    if ext == ".json" {
        Some(raw)
    } else {
        Some(strip_gap_markers(&raw))
    }
}

/// Evaluate all checks for one turn against one artifact, producing a
/// `TurnCorrectness` row. A missing artifact fails every check.
fn evaluate_turn(turn_checks: &TurnChecks, artifact: Option<&str>) -> TurnCorrectness {
    let mut results = Vec::with_capacity(turn_checks.checks.len());
    let mut passed = 0usize;

    for check in &turn_checks.checks {
        let (pass, reason) = match artifact {
            Some(text) => evaluate_check(check, text),
            None => (false, "artifact not found".to_string()),
        };
        if pass {
            passed += 1;
        }
        results.push(CheckResult {
            kind: check.kind().to_string(),
            pass,
            reason,
        });
    }

    TurnCorrectness {
        turn: turn_checks.turn,
        passed,
        total: turn_checks.checks.len(),
        checks: results,
    }
}

/// Compute pass rate (passed / total over all checks) for a set of turns.
fn pass_rate(per_turn: &[TurnCorrectness]) -> f64 {
    let total: usize = per_turn.iter().map(|t| t.total).sum();
    if total == 0 {
        return 0.0;
    }
    let passed: usize = per_turn.iter().map(|t| t.passed).sum();
    round4(passed as f64 / total as f64)
}

/// Score correctness checks for a single experiment directory.
///
/// For each turn `N` starting at 1, reads `checks/turn-N.json` (breaking when
/// missing), evaluates the checks against both the GAP and BASE artifacts, and
/// writes a `correctness` object into `metrics.json` (read-modify-write,
/// mirroring `scorer::score_experiment`).
pub fn score_checks(exp_dir: &Path) -> Result<()> {
    let metrics_path = exp_dir.join("metrics.json");
    if !metrics_path.exists() {
        return Ok(());
    }

    let raw = fs::read_to_string(&metrics_path)?;
    let mut metrics: serde_json::Value = serde_json::from_str(&raw)?;

    let format = metrics["format"].as_str().unwrap_or("text/html");
    let ext = format_to_ext(format);

    let checks_dir = exp_dir.join("checks");
    let gap_dir = exp_dir.join("outputs/gap");
    let base_dir = exp_dir.join("outputs/base");

    let mut gap_per_turn = Vec::new();
    let mut base_per_turn = Vec::new();

    for turn in 1.. {
        let checks_path = checks_dir.join(format!("turn-{turn}.json"));
        if !checks_path.exists() {
            break;
        }

        let checks_raw = fs::read_to_string(&checks_path)?;
        let turn_checks: TurnChecks = serde_json::from_str(&checks_raw)?;

        let gap_artifact = read_artifact(&gap_dir, turn, ext);
        let base_artifact = read_artifact(&base_dir, turn, ext);

        gap_per_turn.push(evaluate_turn(&turn_checks, gap_artifact.as_deref()));
        base_per_turn.push(evaluate_turn(&turn_checks, base_artifact.as_deref()));
    }

    if gap_per_turn.is_empty() {
        return Ok(());
    }

    let correctness = Correctness {
        pass_rate: pass_rate(&gap_per_turn),
        base_pass_rate: pass_rate(&base_per_turn),
        per_turn: gap_per_turn,
    };

    metrics["correctness"] = serde_json::to_value(&correctness)?;
    fs::write(&metrics_path, serde_json::to_string_pretty(&metrics)?)?;

    Ok(())
}

/// Score correctness checks for all experiments in a directory.
pub fn score_checks_all(dir: &Path) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir() && e.path().join("metrics.json").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let id = entry.file_name().to_string_lossy().to_string();
        match score_checks(&entry.path()) {
            Ok(()) => eprintln!("checked {id}"),
            Err(e) => eprintln!("skip {id}: {e}"),
        }
    }

    Ok(())
}
