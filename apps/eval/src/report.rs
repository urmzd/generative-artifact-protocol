//! Report generation from experiment metrics.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn generate(experiments_dir: &Path, format: &str, output: Option<&Path>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(experiments_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("metrics.json").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut all_metrics: Vec<serde_json::Value> = Vec::new();
    for entry in &entries {
        let raw = fs::read_to_string(entry.path().join("metrics.json"))?;
        let m: serde_json::Value = serde_json::from_str(&raw)?;
        all_metrics.push(m);
    }

    if all_metrics.is_empty() {
        eprintln!("no experiments with metrics found");
        return Ok(());
    }

    let out_str = match format {
        "json" => serde_json::to_string_pretty(&all_metrics)?,
        _ => format_human_table(&all_metrics),
    };

    match output {
        Some(path) => fs::write(path, &out_str)?,
        None => std::io::stdout().write_all(out_str.as_bytes())?,
    }

    Ok(())
}

fn format_human_table(metrics: &[serde_json::Value]) -> String {
    let mut out = String::new();

    let model = metrics[0]["model"].as_str().unwrap_or("?");
    out.push_str(&format!("# GAP Experiment Results\n\n"));
    out.push_str(&format!("**Model:** `{model}` | **Experiments:** {}\n\n", metrics.len()));

    out.push_str("| Experiment | Fmt | Base Out | GAP Out | Out Δ | Parse | Apply | Seq Sim | F1 |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");

    let mut total_base_out: u64 = 0;
    let mut total_gap_out: u64 = 0;
    let mut total_parse_ok: usize = 0;
    let mut total_parse_total: usize = 0;
    let mut total_apply_ok: usize = 0;
    let mut total_apply_total: usize = 0;

    for m in metrics {
        let id = m["experiment_id"].as_str().unwrap_or("?");
        let fmt = m["format"].as_str().unwrap_or("?");
        let fmt_short = &fmt[..fmt.len().min(10)];

        let base_out = m["default_flow"]["total_output_tokens"].as_u64().unwrap_or(0);
        let gap_out = m["gap_flow"]["total_output_tokens"].as_u64().unwrap_or(0);
        let out_delta = if base_out > 0 {
            format!("{:.1}%", (1.0 - gap_out as f64 / base_out as f64) * 100.0)
        } else {
            "—".into()
        };

        total_base_out += base_out;
        total_gap_out += gap_out;

        // Parse/apply rates from per-turn data
        let gap_turns = m["gap_flow"]["per_turn"].as_array();
        let (parse_ok, apply_ok, _n_turns) = if let Some(turns) = gap_turns {
            let n = turns.len();
            let p = turns.iter().filter(|t| t["envelope_parsed"].as_bool() == Some(true)).count();
            let a = turns.iter().filter(|t| t["apply_succeeded"].as_bool() == Some(true)).count();
            total_parse_ok += p;
            total_apply_ok += a;
            total_parse_total += n;
            total_apply_total += n;
            (format!("{p}/{n}"), format!("{a}/{n}"), n)
        } else {
            ("—".into(), "—".into(), 0)
        };

        let seq_sim = m["quality"]["mean_sequence_similarity"]
            .as_f64()
            .map(|v| format!("{v:.3}"))
            .unwrap_or("—".into());
        let f1 = m["quality"]["mean_token_f1"]
            .as_f64()
            .map(|v| format!("{v:.3}"))
            .unwrap_or("—".into());

        out.push_str(&format!(
            "| {id} | {fmt_short} | {base_out:>6} | {gap_out:>6} | {out_delta:>6} | {parse_ok} | {apply_ok} | {seq_sim} | {f1} |\n"
        ));
    }

    let total_delta = if total_base_out > 0 {
        format!("{:.1}%", (1.0 - total_gap_out as f64 / total_base_out as f64) * 100.0)
    } else {
        "—".into()
    };

    out.push_str(&format!(
        "\n**Totals:** Output savings: {total_delta} | Parse: {total_parse_ok}/{total_parse_total} | Apply: {total_apply_ok}/{total_apply_total}\n"
    ));

    out
}
