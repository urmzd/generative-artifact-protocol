//! Content quality scoring — LCS, token F1, ROUGE-L.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::experiment::format_to_ext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnQuality {
    pub turn: usize,
    pub sequence_similarity: f64,
    pub token_f1: f64,
    pub rouge_l: f64,
    pub base_char_count: usize,
    pub gap_char_count: usize,
    pub char_delta_pct: f64,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentQuality {
    pub per_turn: Vec<TurnQuality>,
    pub mean_sequence_similarity: f64,
    pub mean_token_f1: f64,
    pub mean_rouge_l: f64,
}

/// Strip `<gap:target ...>` and `</gap:target>` markers.
fn strip_gap_markers(text: &str) -> String {
    let re = Regex::new(r"</?gap:target[^>]*>").unwrap();
    re.replace_all(text, "").to_string()
}

/// Character-level LCS length (space-optimized).
fn lcs_len(a: &[char], b: &[char]) -> usize {
    let (m, n) = (a.len(), b.len());
    if m == 0 || n == 0 {
        return 0;
    }
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        for j in 1..=n {
            curr[j] = if a[i - 1] == b[j - 1] {
                prev[j - 1] + 1
            } else {
                prev[j].max(curr[j - 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }
    prev[n]
}

/// Sequence similarity: 2 * LCS / (|a| + |b|).
fn sequence_similarity(a: &str, b: &str) -> f64 {
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let total = ac.len() + bc.len();
    if total == 0 {
        return 1.0;
    }
    2.0 * lcs_len(&ac, &bc) as f64 / total as f64
}

/// Word-level tokenization.
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.to_lowercase())
        .collect()
}

/// Token F1: word-level precision/recall F1.
fn token_f1(reference: &str, hypothesis: &str) -> f64 {
    let ref_tokens = tokenize(reference);
    let hyp_tokens = tokenize(hypothesis);

    if ref_tokens.is_empty() && hyp_tokens.is_empty() {
        return 1.0;
    }
    if ref_tokens.is_empty() || hyp_tokens.is_empty() {
        return 0.0;
    }

    let ref_counts: HashMap<&str, usize> =
        ref_tokens.iter().fold(HashMap::new(), |mut m, t| { *m.entry(t.as_str()).or_default() += 1; m });
    let hyp_counts: HashMap<&str, usize> =
        hyp_tokens.iter().fold(HashMap::new(), |mut m, t| { *m.entry(t.as_str()).or_default() += 1; m });

    let common: usize = ref_counts
        .iter()
        .map(|(t, &rc)| rc.min(*hyp_counts.get(t).unwrap_or(&0)))
        .sum();

    let precision = common as f64 / hyp_tokens.len() as f64;
    let recall = common as f64 / ref_tokens.len() as f64;

    if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

/// Word-level LCS length.
fn word_lcs_len(a: &[String], b: &[String]) -> usize {
    let (m, n) = (a.len(), b.len());
    if m == 0 || n == 0 {
        return 0;
    }
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        for j in 1..=n {
            curr[j] = if a[i - 1] == b[j - 1] {
                prev[j - 1] + 1
            } else {
                prev[j].max(curr[j - 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }
    prev[n]
}

/// ROUGE-L: word-level LCS F1.
fn rouge_l(reference: &str, hypothesis: &str) -> f64 {
    let ref_words = tokenize(reference);
    let hyp_words = tokenize(hypothesis);

    if ref_words.is_empty() && hyp_words.is_empty() {
        return 1.0;
    }
    if ref_words.is_empty() || hyp_words.is_empty() {
        return 0.0;
    }

    let lcs = word_lcs_len(&ref_words, &hyp_words);
    let precision = lcs as f64 / hyp_words.len() as f64;
    let recall = lcs as f64 / ref_words.len() as f64;

    if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

/// Line diff stats.
fn line_diff(base: &str, gap: &str) -> (usize, usize) {
    let base_lines: HashMap<&str, usize> =
        base.lines().fold(HashMap::new(), |mut m, l| { *m.entry(l).or_default() += 1; m });
    let gap_lines: HashMap<&str, usize> =
        gap.lines().fold(HashMap::new(), |mut m, l| { *m.entry(l).or_default() += 1; m });

    let mut added = 0usize;
    let mut removed = 0usize;

    for (line, &count) in &gap_lines {
        let base_count = base_lines.get(line).copied().unwrap_or(0);
        if count > base_count {
            added += count - base_count;
        }
    }
    for (line, &count) in &base_lines {
        let gap_count = gap_lines.get(line).copied().unwrap_or(0);
        if count > gap_count {
            removed += count - gap_count;
        }
    }

    (added, removed)
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// Score a single experiment directory. Updates metrics.json with quality data.
pub fn score_experiment(exp_dir: &Path) -> Result<()> {
    let metrics_path = exp_dir.join("metrics.json");
    if !metrics_path.exists() {
        return Ok(());
    }

    let raw = fs::read_to_string(&metrics_path)?;
    let mut metrics: serde_json::Value = serde_json::from_str(&raw)?;

    let format = metrics["format"].as_str().unwrap_or("text/html");
    let ext = format_to_ext(format);

    let base_dir = exp_dir.join("outputs/base");
    let gap_dir = exp_dir.join("outputs/gap");

    let mut per_turn = Vec::new();

    for turn in 1.. {
        let base_path = base_dir.join(format!("turn-{turn}{ext}"));
        let gap_path = gap_dir.join(format!("turn-{turn}{ext}"));

        if !base_path.exists() || !gap_path.exists() {
            break;
        }

        let base_text = fs::read_to_string(&base_path)?;
        let gap_raw = fs::read_to_string(&gap_path)?;
        let gap_text = strip_gap_markers(&gap_raw);

        let seq_sim = round4(sequence_similarity(&base_text, &gap_text));
        let f1 = round4(token_f1(&base_text, &gap_text));
        let rl = round4(rouge_l(&base_text, &gap_text));

        let base_chars = base_text.len();
        let gap_chars = gap_text.len();
        let char_delta = if base_chars > 0 {
            ((gap_chars as f64 - base_chars as f64) / base_chars as f64 * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };

        let (lines_added, lines_removed) = line_diff(&base_text, &gap_text);

        per_turn.push(TurnQuality {
            turn,
            sequence_similarity: seq_sim,
            token_f1: f1,
            rouge_l: rl,
            base_char_count: base_chars,
            gap_char_count: gap_chars,
            char_delta_pct: char_delta,
            lines_added,
            lines_removed,
        });
    }

    if per_turn.is_empty() {
        return Ok(());
    }

    let n = per_turn.len() as f64;
    let quality = ExperimentQuality {
        mean_sequence_similarity: round4(per_turn.iter().map(|t| t.sequence_similarity).sum::<f64>() / n),
        mean_token_f1: round4(per_turn.iter().map(|t| t.token_f1).sum::<f64>() / n),
        mean_rouge_l: round4(per_turn.iter().map(|t| t.rouge_l).sum::<f64>() / n),
        per_turn,
    };

    metrics["quality"] = serde_json::to_value(&quality)?;
    fs::write(&metrics_path, serde_json::to_string_pretty(&metrics)?)?;

    Ok(())
}

/// Score all experiments in a directory.
pub fn score_all(experiments_dir: &Path) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(experiments_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir() && e.path().join("metrics.json").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let id = entry.file_name().to_string_lossy().to_string();
        match score_experiment(&entry.path()) {
            Ok(()) => eprintln!("scored {id}"),
            Err(e) => eprintln!("skip {id}: {e}"),
        }
    }

    Ok(())
}
