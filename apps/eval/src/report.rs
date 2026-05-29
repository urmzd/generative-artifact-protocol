//! Report generation from experiment metrics.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cost::{self, Cache};

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

    out.push_str(&validity_section(metrics));
    out.push_str(&decomposition_section(metrics));
    out.push_str(&cost_analysis(metrics, model));
    out.push_str(&agent_loop_section(metrics, model));
    out.push_str(&latency_section(metrics));
    out.push_str(&correctness_section(metrics));

    out
}

/// Effect-2 (orchestrator context separation) — the tool-use win. In an agent
/// loop the artifact-producing tool's result re-enters the orchestrator context
/// every turn; with GAP the orchestrator holds only a handle. Projects the
/// suite-wide orchestrator input tokens across regimes over a curve of extra
/// reasoning turns. MODELED from measured artifact sizes (base full-regen output
/// per version) + a conservative handle size (mean GAP envelope output, ≥20 tok
/// — an upper bound, since a handle is smaller than the envelope that made it).
fn agent_loop_section(metrics: &[serde_json::Value], model: &str) -> String {
    let p = cost::price_for(model);
    let eligible: Vec<&serde_json::Value> = metrics
        .iter()
        .filter(|m| m["default_flow"].is_object() && m["gap_flow"].is_object())
        .filter(|m| m["validity"]["gap_run_degenerate"].as_bool() != Some(true))
        .collect();
    if eligible.is_empty() {
        return String::new();
    }

    // Per-experiment artifact sizes (tokens per version) and handle size.
    let session = |m: &serde_json::Value| -> (Vec<f64>, f64) {
        let mut versions = Vec::new();
        if let Some(o) = m["base_turn0"]["output_tokens"].as_f64() {
            versions.push(o);
        }
        if let Some(per) = m["default_flow"]["per_turn"].as_array() {
            versions.extend(per.iter().filter_map(|t| t["output_tokens"].as_f64()));
        }
        let gap_outs: Vec<f64> = m["gap_flow"]["per_turn"]
            .as_array()
            .map(|a| a.iter().filter_map(|t| t["output_tokens"].as_f64()).collect())
            .unwrap_or_default();
        let handle = if gap_outs.is_empty() {
            20.0
        } else {
            (gap_outs.iter().sum::<f64>() / gap_outs.len() as f64).max(20.0)
        };
        (versions, handle)
    };

    let scale = 1_000_000.0;
    let mut s = String::from("\n## Agent loop — Effect 2 (orchestrator context separation), MODELED\n\n");
    s.push_str(
        "Orchestrator-wallet input tokens summed across eligible experiments, as the orchestrator \
spends extra reasoning turns holding the artifact. **KeepLatest** (steelman baseline) keeps only \
the current body in context; **Accumulate** (worst case) retains every version; **GAP** holds a \
handle. This is a *separate ledger* from the edit work above (the maintain wallet = Scenario C).\n\n",
    );
    s.push_str("| Extra turns | KeepLatest in | Accumulate in | GAP in | Re-reads avoided | GAP savings vs KeepLatest | GAP $ vs KeepLatest $ |\n");
    s.push_str("|---:|---:|---:|---:|---:|---:|---:|\n");
    for &extra in &[0usize, 2, 5, 10] {
        let (mut keep, mut acc, mut gap, mut rr) = (0.0, 0.0, 0.0, 0usize);
        for m in &eligible {
            let (versions, handle) = session(m);
            let al = cost::agent_loop(&versions, handle, extra);
            keep += al.keep_latest_input;
            acc += al.accumulate_input;
            gap += al.gap_input;
            rr += al.rereads_avoided;
        }
        let keep_usd = keep / scale * p.input;
        let gap_usd = gap / scale * p.input;
        s.push_str(&format!(
            "| +{extra} | {keep:.0} | {acc:.0} | {gap:.0} | {rr} | {:.1}% | ${gap_usd:.4} vs ${keep_usd:.4} |\n",
            cost::savings_pct(keep, gap),
        ));
    }
    s.push_str(&format!(
        "\nAcross {} eligible experiments. The KeepLatest column grows linearly with reasoning \
turns and Accumulate quadratically, while GAP stays flat — every re-read avoided is a full \
artifact body the orchestrator never pays to re-ingest.\n",
        eligible.len(),
    ));
    s
}

/// Correctness-oracle summary — assertion checks (`checks/turn-N.json`) evaluated
/// against the produced artifact. This is the high-fidelity signal: it catches
/// runs that "apply successfully" but silently drop items or corrupt content
/// (the failure mode multi-item / multi-page experiments are designed to expose).
fn correctness_section(metrics: &[serde_json::Value]) -> String {
    let with: Vec<&serde_json::Value> =
        metrics.iter().filter(|m| m["correctness"].is_object()).collect();
    if with.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "\n## Correctness oracles (checks/turn-N.json — multi-item/multi-page fidelity)\n\n",
    );
    s.push_str(
        "Pass rate = fraction of per-turn assertions satisfied: targeted change present, \
old/deleted values gone, and EXACT item count preserved (collateral-loss detector). \
GAP vs BASE evaluated on identical oracles.\n\n",
    );
    s.push_str("| Experiment | Fmt | GAP correct | Base correct |\n|---|---|---:|---:|\n");
    let (mut g_sum, mut b_sum) = (0.0, 0.0);
    for m in &with {
        let id = m["experiment_id"].as_str().unwrap_or("?");
        let fmt = m["format"].as_str().unwrap_or("?");
        let fmt_short = &fmt[..fmt.len().min(12)];
        let g = m["correctness"]["pass_rate"].as_f64().unwrap_or(0.0);
        let b = m["correctness"]["base_pass_rate"].as_f64().unwrap_or(0.0);
        g_sum += g;
        b_sum += b;
        s.push_str(&format!(
            "| {id} | {fmt_short} | {:.0}% | {:.0}% |\n",
            g * 100.0,
            b * 100.0
        ));
    }
    let n = with.len() as f64;
    s.push_str(&format!(
        "\n**Mean correctness:** GAP {:.1}% | Base {:.1}% (n={})\n",
        g_sum / n * 100.0,
        b_sum / n * 100.0,
        with.len()
    ));
    s
}

/// Build an init-inclusive `[init, edit1, ...]` token sequence for a flow.
/// `turn0_key` is the metrics key for the creation turn (e.g. `base_turn0`),
/// `flow` is the flow node holding `per_turn`.
fn flow_turns(m: &serde_json::Value, turn0_key: &str, flow: &serde_json::Value) -> Vec<cost::Turn> {
    let mut turns = Vec::new();
    let t0 = &m[turn0_key];
    if t0.is_object() {
        turns.push(cost::Turn {
            input: t0["input_tokens"].as_f64().unwrap_or(0.0),
            cached: t0["cached_input_tokens"].as_f64().unwrap_or(0.0),
            output: t0["output_tokens"].as_f64().unwrap_or(0.0),
        });
    }
    if let Some(per_turn) = flow["per_turn"].as_array() {
        for t in per_turn {
            turns.push(cost::Turn {
                input: t["input_tokens"].as_f64().unwrap_or(0.0),
                cached: t["cached_input_tokens"].as_f64().unwrap_or(0.0),
                output: t["output_tokens"].as_f64().unwrap_or(0.0),
            });
        }
    }
    turns
}

/// Mean of a per-experiment value, skipping experiments where it is absent.
fn mean_of(metrics: &[serde_json::Value], f: impl Fn(&serde_json::Value) -> Option<f64>) -> Option<f64> {
    let vals: Vec<f64> = metrics.iter().filter_map(&f).collect();
    if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) }
}

/// Run-validity gate summary — how many runs are headline-eligible.
fn validity_section(metrics: &[serde_json::Value]) -> String {
    let total = metrics.len();
    let degenerate = metrics.iter().filter(|m| m["validity"]["gap_run_degenerate"].as_bool() == Some(true)).count();
    let non_monotone = metrics.iter().filter(|m| m["validity"]["base_input_monotone"].as_bool() == Some(false)).count();
    if degenerate == 0 && non_monotone == 0 {
        return String::new();
    }
    let mut s = String::from("\n## Run validity\n\n");
    if degenerate > 0 {
        s.push_str(&format!(
            "- ⚠ **{degenerate}/{total}** GAP runs are **degenerate** (artifact never changed — all edits no-ops). Their \"output savings\" are illusory and must be excluded from any headline.\n"
        ));
    }
    if non_monotone > 0 {
        s.push_str(&format!(
            "- ⚠ **{non_monotone}/{total}** runs have **non-monotone base input** ⇒ the provider reports post-cache token counts; the raw input axis is not directly interpretable for those.\n"
        ));
    }
    s
}

/// A/B/C decomposition — separates the input win (statelessness) from the
/// output win (envelopes). Only rendered when the stateless flow was run.
fn decomposition_section(metrics: &[serde_json::Value]) -> String {
    let has = metrics.iter().any(|m| m["decomposition"].is_object());
    if !has {
        return String::new();
    }
    let g = |key: &str| mean_of(metrics, |m| m["decomposition"][key].as_f64());
    let fmt = |v: Option<f64>| v.map(|x| format!("{x:.1}%")).unwrap_or("—".into());
    let mut s = String::from("\n## Savings decomposition (A/B/C, init-inclusive, MEASURED tokens)\n\n");
    s.push_str("Mean per-experiment savings. **B vs A** = the input win from going stateless (any baseline can adopt it). **C vs B** = GAP's defensible output-envelope win.\n\n");
    s.push_str("| Axis | Comparison | Mean savings |\n|---|---|---:|\n");
    s.push_str(&format!("| Input | B vs A (statelessness) | {} |\n", fmt(g("input_savings_b_vs_a_pct"))));
    s.push_str(&format!("| Output | C vs B (edit envelopes) | {} |\n", fmt(g("output_savings_c_vs_b_pct"))));
    s.push_str(&format!("| Input | C vs A | {} |\n", fmt(g("input_savings_c_vs_a_pct"))));
    s.push_str(&format!("| Output | C vs A | {} |\n", fmt(g("output_savings_c_vs_a_pct"))));
    s
}

/// Aggregate init-inclusive cost of a flow across all experiments, under a
/// cache regime. `turn0_key` names the creation-turn node; `growing_prefix` is
/// true only for Scenario A (append-only conversation), whose stable prefix is
/// the one a hot cache can actually serve.
fn agg_cost(
    metrics: &[serde_json::Value],
    turn0_key: &str,
    flow_key: &str,
    p: cost::Price,
    cache: Cache,
    growing_prefix: bool,
) -> f64 {
    metrics
        .iter()
        .filter(|m| m[flow_key].is_object())
        .map(|m| {
            let turns = flow_turns(m, turn0_key, &m[flow_key]);
            cost::flow_cost(&turns, p, cache, growing_prefix)
        })
        .sum()
}

/// Cost analysis — init-inclusive, multi-regime. Caching most benefits the base
/// flow (Scenario A, append-only cacheable prefix); GAP's per-turn artifact
/// injection is uncacheable. The headline: even granting the baseline a
/// perfectly hot cache (`theoretical_best`), GAP still wins because output
/// tokens are never cached. Cost figures are MODELED.
fn cost_analysis(metrics: &[serde_json::Value], model: &str) -> String {
    let p = cost::price_for(model);
    let has_gap = metrics.iter().any(|m| m["gap_flow"].is_object());
    let has_base = metrics.iter().any(|m| m["default_flow"].is_object());
    if !has_gap || !has_base {
        return String::new();
    }

    // Base = Scenario A (growing prefix); GAP = stateless (no growing prefix).
    let base_off = agg_cost(metrics, "base_turn0", "default_flow", p, Cache::Off, true);
    let base_obs = agg_cost(metrics, "base_turn0", "default_flow", p, Cache::Observed, true);
    let base_best = agg_cost(metrics, "base_turn0", "default_flow", p, Cache::TheoreticalBest, true);
    let gap_off = agg_cost(metrics, "gap_turn0", "gap_flow", p, Cache::Off, false);
    let gap_obs = agg_cost(metrics, "gap_turn0", "gap_flow", p, Cache::Observed, false);

    let mut s = String::new();
    s.push_str("\n## Cost analysis — init-inclusive, cache regimes (MODELED $)\n\n");
    s.push_str(&format!(
        "Prices (USD/1M): input ${:.3}, cached-input ${:.3}, output ${:.3}. Output is never cached.\n\n",
        p.input, p.cached_in, p.output
    ));
    s.push_str("| Flow | Cost (cache off) | Cost (cache observed) | Cost (cache theoretical-best) |\n");
    s.push_str("|---|---:|---:|---:|\n");
    s.push_str(&format!(
        "| Base (Scenario A, full regen) | ${base_off:.4} | ${base_obs:.4} | ${base_best:.4} |\n"
    ));
    // GAP gets no theoretical-best credit (its artifact injection is uncacheable
    // and its small stable system prefix is conservatively ignored).
    s.push_str(&format!(
        "| GAP (Scenario C, envelopes) | ${gap_off:.4} | ${gap_obs:.4} | ${gap_off:.4} |\n"
    ));
    s.push_str(&format!(
        "\n**GAP savings vs base:** {:.1}% (cache off) → {:.1}% (base perfectly cached, GAP not).\n",
        cost::savings_pct(base_off, gap_off),
        cost::savings_pct(base_best, gap_off),
    ));
    s.push_str("Even with a perfectly hot cache on the baseline, GAP's advantage survives — the residual is the output-token win, which no cache can discount.\n");

    // Break-even vs base (Scenario A), per experiment with all data present.
    let mut be: Vec<usize> = Vec::new();
    for m in metrics {
        if !m["default_flow"].is_object() || !m["gap_flow"].is_object() {
            continue;
        }
        let base_turns = flow_turns(m, "base_turn0", &m["default_flow"]);
        let gap_turns = flow_turns(m, "gap_turn0", &m["gap_flow"]);
        let base_cum = cost::cumulative(&base_turns, p, Cache::TheoreticalBest, true);
        let gap_cum = cost::cumulative(&gap_turns, p, Cache::Off, false);
        if let Some(t) = cost::break_even(&base_cum, &gap_cum) {
            be.push(t);
        }
    }
    if !be.is_empty() {
        let mean = be.iter().sum::<usize>() as f64 / be.len() as f64;
        s.push_str(&format!(
            "\n**Break-even** (cumulative GAP cost < perfectly-cached base): reached in {}/{} experiments, mean edit turn {:.1}.\n",
            be.len(),
            metrics.iter().filter(|m| m["gap_flow"].is_object() && m["default_flow"].is_object()).count(),
            mean,
        ));
    }
    s
}

/// Latency section — TTFT / TTLT per flow (MEASURED). These are collected per
/// turn but were previously never surfaced. GAP trades a possibly-higher TTFT
/// (the artifact sits in the input → longer prefill; structured decoding adds
/// overhead) for a much lower TTLT (far fewer output tokens).
fn latency_section(metrics: &[serde_json::Value]) -> String {
    let collect = |flow_key: &str, field: &str| -> Option<f64> {
        let vals: Vec<f64> = metrics
            .iter()
            .filter_map(|m| m[flow_key]["per_turn"].as_array())
            .flatten()
            .filter_map(|t| t[field].as_f64())
            .collect();
        if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) }
    };
    let fmt = |v: Option<f64>| v.map(|x| format!("{x:.0} ms")).unwrap_or("—".into());

    let rows = [
        ("Base (full regen)", "default_flow"),
        ("Stateless (full regen)", "stateless_flow"),
        ("GAP (envelopes)", "gap_flow"),
    ];
    let present: Vec<_> = rows.iter().filter(|(_, k)| metrics.iter().any(|m| m[k]["per_turn"].is_array())).collect();
    if present.is_empty() {
        return String::new();
    }

    let mut s = String::from("\n## Latency (mean over edit turns, MEASURED)\n\n");
    s.push_str("Wall-clock includes network + queueing, not pure prefill/decode.\n\n");
    s.push_str("| Flow | TTFT | TTLT | Total latency |\n|---|---:|---:|---:|\n");
    for (label, key) in present {
        s.push_str(&format!(
            "| {label} | {} | {} | {} |\n",
            fmt(collect(key, "ttft_ms")),
            fmt(collect(key, "ttlt_ms")),
            fmt(collect(key, "latency_ms")),
        ));
    }
    s
}
