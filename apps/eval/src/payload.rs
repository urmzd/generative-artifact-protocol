//! Payload-size report over the apply-engine fixtures.
//!
//! Wire bytes = compact JSON serialization of the full envelope (what a
//! producer must emit on the wire); content bytes = the replacement text
//! carried inside ops. Savings compare an edit envelope's wire bytes against
//! the same case's synthesize envelope (the wire cost of a full
//! regeneration). Envelopes whose targets do not resolve against their case's
//! artifact are counted but excluded from the medians — a payload that cannot
//! be applied saves nothing.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use gap::apply::{apply_edit, TextResolver};
use gap::gap::{EditOp, Envelope};

const EDIT_FAMILIES: &[&str] = &[
    "edit-replace",
    "edit-multi",
    "edit-delete",
    "edit-section-single",
    "edit-section-multi",
];

#[derive(Default)]
struct FamilyStats {
    /// (wire bytes, content bytes, % of the case's synthesize wire bytes).
    samples: Vec<(usize, usize, f64)>,
    rejected: usize,
}

fn median(vals: &mut [f64]) -> f64 {
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = vals.len() / 2;
    if vals.len().is_multiple_of(2) {
        (vals[mid - 1] + vals[mid]) / 2.0
    } else {
        vals[mid]
    }
}

fn read_envelopes(dir: &Path, name: &str) -> Result<Vec<Envelope>> {
    let path = dir.join("envelopes").join(format!("{name}.jsonl"));
    let raw = fs::read_to_string(&path).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| Envelope::from_json(l).with_context(|| format!("parse {}", path.display())))
        .collect()
}

fn content_bytes(env: &Envelope) -> usize {
    env.content
        .iter()
        .map(|item| {
            // Synthesize items carry `body`; edit ops carry `content`.
            item["body"]
                .as_str()
                .or_else(|| item["content"].as_str())
                .map_or(0, str::len)
        })
        .sum()
}

pub fn generate(fixtures_dir: &Path) -> Result<()> {
    let mut cases: Vec<String> = fs::read_dir(fixtures_dir)
        .with_context(|| format!("read fixtures dir {}", fixtures_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    cases.sort();
    anyhow::ensure!(!cases.is_empty(), "no fixture cases found");

    let resolver = TextResolver {
        format: "text/html".to_string(),
    };

    let mut synth = FamilyStats::default();
    let mut families: Vec<(&str, FamilyStats)> = EDIT_FAMILIES
        .iter()
        .map(|f| (*f, FamilyStats::default()))
        .collect();

    for case in &cases {
        let dir = fixtures_dir.join(case);
        let artifact = fs::read_to_string(dir.join("artifacts/dashboard.html"))
            .with_context(|| format!("read artifact for case {case}"))?;

        let synth_envs = read_envelopes(&dir, "synthesize")?;
        let synth_env = synth_envs
            .first()
            .with_context(|| format!("case {case}: missing synthesize envelope"))?;
        let synth_wire = serde_json::to_string(synth_env)?.len();
        synth
            .samples
            .push((synth_wire, content_bytes(synth_env), 100.0));

        for (family, stats) in &mut families {
            for env in read_envelopes(&dir, family)? {
                let wire = serde_json::to_string(&env)?.len();
                let content = content_bytes(&env);
                let ops: Vec<EditOp> = env
                    .content
                    .iter()
                    .cloned()
                    .map(serde_json::from_value)
                    .collect::<Result<_, _>>()
                    .with_context(|| format!("case {case}: parse {family} ops"))?;
                if apply_edit(&resolver, &artifact, &ops).is_ok() {
                    stats
                        .samples
                        .push((wire, content, wire as f64 / synth_wire as f64 * 100.0));
                } else {
                    stats.rejected += 1;
                }
            }
        }
    }

    println!(
        "# GAP payload sizes ({} apply-engine fixture cases)\n",
        cases.len()
    );
    println!(
        "Wire bytes = compact JSON serialization of the full envelope; content bytes = \
replacement text inside ops. \"% of synthesize\" compares an edit envelope's wire bytes \
against the same case's synthesize envelope (the wire cost of a full regeneration). \
Medians across all applying envelopes.\n"
    );
    println!("| Family | Envelopes (apply ok) | Median wire B | Median content B | Median % of synthesize | Median savings |");
    println!("|---|---:|---:|---:|---:|---:|");

    let row = |name: &str, stats: &FamilyStats| {
        let total = stats.samples.len() + stats.rejected;
        if stats.samples.is_empty() {
            println!("| {name} | 0/{total} | — | — | — | — |");
            return;
        }
        let mut wires: Vec<f64> = stats.samples.iter().map(|s| s.0 as f64).collect();
        let mut contents: Vec<f64> = stats.samples.iter().map(|s| s.1 as f64).collect();
        let mut pcts: Vec<f64> = stats.samples.iter().map(|s| s.2).collect();
        let pct = median(&mut pcts);
        let savings = if name == "synthesize" {
            "—".to_string()
        } else {
            format!("{:.1}%", 100.0 - pct)
        };
        println!(
            "| {name} | {}/{total} | {:.0} | {:.0} | {pct:.1}% | {savings} |",
            stats.samples.len(),
            median(&mut wires),
            median(&mut contents),
        );
    };

    row("synthesize", &synth);
    for (family, stats) in &families {
        row(family, stats);
    }

    let rejected: usize = families.iter().map(|(_, s)| s.rejected).sum();
    if rejected > 0 {
        println!(
            "\n⚠ {rejected} edit envelope(s) target raw source lines instead of `<gap:target>` \
ids and never resolve (legacy fixtures); they are excluded from the medians above."
        );
    }

    Ok(())
}
