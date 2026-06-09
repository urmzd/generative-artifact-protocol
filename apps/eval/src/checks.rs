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

use crate::experiment::{format_to_ext, strip_gap_markers};

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

        Check::RegexCount {
            pattern, expected, ..
        } => {
            let re = match Regex::new(pattern) {
                Ok(re) => re,
                Err(e) => return (false, format!("invalid regex {pattern:?}: {e}")),
            };
            let count = re.find_iter(artifact).count();
            if count == *expected {
                (true, format!("{pattern:?} matched {count} time(s)"))
            } else {
                (
                    false,
                    format!("{pattern:?} matched {count} time(s), expected {expected}"),
                )
            }
        }

        Check::RegexCountAtLeast { pattern, min, .. } => {
            let re = match Regex::new(pattern) {
                Ok(re) => re,
                Err(e) => return (false, format!("invalid regex {pattern:?}: {e}")),
            };
            let count = re.find_iter(artifact).count();
            if count >= *min {
                (
                    true,
                    format!("{pattern:?} matched {count} time(s) (>= {min})"),
                )
            } else {
                (
                    false,
                    format!("{pattern:?} matched {count} time(s), expected >= {min}"),
                )
            }
        }

        Check::JsonPointerEquals { pointer, value, .. } => {
            let parsed: serde_json::Value = match serde_json::from_str(artifact) {
                Ok(v) => v,
                Err(e) => return (false, format!("not valid JSON: {e}")),
            };
            match parsed.pointer(pointer) {
                Some(actual) if actual == value => (true, format!("{pointer} == {value}")),
                Some(actual) => (false, format!("{pointer} == {actual}, expected {value}")),
                None => (false, format!("{pointer} not present")),
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pass(check: &Check, artifact: &str) -> bool {
        evaluate_check(check, artifact).0
    }

    #[test]
    fn valid_json_check() {
        let c = Check::ValidJson { desc: None };
        assert!(pass(&c, r#"{"a": 1}"#));
        assert!(pass(&c, "[1, 2, 3]"));
        assert!(!pass(&c, "<html></html>"));
        assert!(!pass(&c, r#"{"a": 1,}"#));
    }

    #[test]
    fn contains_check() {
        let c = Check::Contains {
            value: "<h1>".into(),
            desc: None,
        };
        assert!(pass(&c, "<h1>Title</h1>"));
        // Literal substring, case-sensitive — no regex semantics.
        assert!(!pass(&c, "<H1>Title</H1>"));
        assert!(!pass(&c, "no heading"));
    }

    #[test]
    fn absent_check() {
        let c = Check::Absent {
            value: "deleted-item".into(),
            desc: None,
        };
        assert!(pass(&c, "clean artifact"));
        assert!(!pass(&c, "still has deleted-item here"));
    }

    #[test]
    fn regex_count_check() {
        let c = Check::RegexCount {
            pattern: "<li>".into(),
            expected: 3,
            desc: None,
        };
        assert!(pass(&c, "<li>a</li><li>b</li><li>c</li>"));
        // Collateral-loss detector: one item dropped must fail.
        assert!(!pass(&c, "<li>a</li><li>b</li>"));
        // Expecting zero matches passes on no match.
        let zero = Check::RegexCount {
            pattern: "x".into(),
            expected: 0,
            desc: None,
        };
        assert!(pass(&zero, "abc"));
    }

    #[test]
    fn regex_count_invalid_pattern_fails_not_panics() {
        let c = Check::RegexCount {
            pattern: "(unclosed".into(),
            expected: 1,
            desc: None,
        };
        let (ok, reason) = evaluate_check(&c, "anything");
        assert!(!ok);
        assert!(reason.contains("invalid regex"), "{reason}");
    }

    #[test]
    fn regex_count_at_least_check() {
        let c = Check::RegexCountAtLeast {
            pattern: r"\d+".into(),
            min: 2,
            desc: None,
        };
        assert!(pass(&c, "1 and 2 and 3"));
        assert!(pass(&c, "1 and 2"));
        assert!(!pass(&c, "only 1"));
    }

    #[test]
    fn json_pointer_equals_strings_and_arrays() {
        let artifact = r#"{"name": "widget", "items": [{"id": "a"}, {"id": "b"}]}"#;
        let eq = |pointer: &str, value: serde_json::Value| Check::JsonPointerEquals {
            pointer: pointer.into(),
            value,
            desc: None,
        };
        assert!(pass(&eq("/name", json!("widget")), artifact));
        assert!(!pass(&eq("/name", json!("gadget")), artifact));
        assert!(pass(&eq("/items/1/id", json!("b")), artifact));
        // Missing pointer fails rather than erroring.
        assert!(!pass(&eq("/missing", json!(1)), artifact));
        // Non-JSON artifact fails every pointer check.
        assert!(!pass(&eq("/name", json!("widget")), "not json"));
    }

    #[test]
    fn json_pointer_equals_numeric_types_are_strict() {
        let artifact = r#"{"age": 30, "score": 1.5}"#;
        let eq = |pointer: &str, value: serde_json::Value| Check::JsonPointerEquals {
            pointer: pointer.into(),
            value,
            desc: None,
        };
        assert!(pass(&eq("/age", json!(30)), artifact));
        assert!(pass(&eq("/score", json!(1.5)), artifact));
        // serde_json numbers are typed: 30 (integer) != 30.0 (float), and a
        // string "30" never equals the number 30. Check authors must match
        // the artifact's JSON type exactly.
        assert!(!pass(&eq("/age", json!(30.0)), artifact));
        assert!(!pass(&eq("/age", json!("30")), artifact));
    }

    #[test]
    fn turn_checks_parse_from_tagged_json() {
        let raw = r#"{
            "turn": 2,
            "checks": [
                {"kind": "contains", "value": "<h1>", "desc": "has a heading"},
                {"kind": "regex_count", "pattern": "<li>", "expected": 4},
                {"kind": "json_pointer_equals", "pointer": "/name", "value": "widget"}
            ]
        }"#;
        let tc: TurnChecks = serde_json::from_str(raw).unwrap();
        assert_eq!(tc.turn, 2);
        let kinds: Vec<&str> = tc.checks.iter().map(|c| c.kind()).collect();
        assert_eq!(
            kinds,
            vec!["contains", "regex_count", "json_pointer_equals"]
        );
    }

    #[test]
    fn missing_artifact_fails_every_check() {
        let tc = TurnChecks {
            turn: 1,
            checks: vec![
                Check::ValidJson { desc: None },
                Check::Contains {
                    value: "x".into(),
                    desc: None,
                },
            ],
        };
        let result = evaluate_turn(&tc, None);
        assert_eq!((result.passed, result.total), (0, 2));
        assert!(result
            .checks
            .iter()
            .all(|c| c.reason == "artifact not found"));
    }

    #[test]
    fn pass_rate_over_all_checks() {
        let tc = TurnChecks {
            turn: 1,
            checks: vec![
                Check::Contains {
                    value: "a".into(),
                    desc: None,
                },
                Check::Contains {
                    value: "z".into(),
                    desc: None,
                },
            ],
        };
        let per_turn = vec![evaluate_turn(&tc, Some("a b c"))];
        assert_eq!(pass_rate(&per_turn), 0.5);
        assert_eq!(pass_rate(&[]), 0.0);
    }
}
