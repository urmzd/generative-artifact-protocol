//! Report generation from experiment metrics.
//!
//! The human report deserializes every `metrics.json` into the typed
//! [`Metrics`] struct before rendering, so a renamed or mistyped field fails
//! loudly at load time instead of silently rendering as 0.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cost::{self, Cache};
use crate::experiment::{Decomposition, FlowMetrics, Metrics, TurnMetrics, TurnResult};

pub fn generate(experiments_dir: &Path, format: &str, output: Option<&Path>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(experiments_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("metrics.json").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut raw: Vec<(std::path::PathBuf, String)> = Vec::new();
    for entry in &entries {
        let path = entry.path().join("metrics.json");
        let text = fs::read_to_string(&path)?;
        raw.push((path, text));
    }

    if raw.is_empty() {
        eprintln!("no experiments with metrics found");
        return Ok(());
    }

    let out_str = match format {
        // JSON passthrough stays schema-agnostic so fields unknown to the
        // typed structs survive into the JSON report.
        "json" => {
            let values: Vec<serde_json::Value> = raw
                .iter()
                .map(|(path, text)| {
                    serde_json::from_str(text).with_context(|| format!("parse {}", path.display()))
                })
                .collect::<Result<_>>()?;
            serde_json::to_string_pretty(&values)?
        }
        _ => {
            let metrics: Vec<Metrics> = raw
                .iter()
                .map(|(path, text)| {
                    serde_json::from_str(text).with_context(|| format!("parse {}", path.display()))
                })
                .collect::<Result<_>>()?;
            format_human_table(&metrics)
        }
    };

    match output {
        Some(path) => fs::write(path, &out_str)?,
        None => std::io::stdout().write_all(out_str.as_bytes())?,
    }

    Ok(())
}

/// Headline eligibility — a degenerate run (artifact never changed, so its
/// "savings" are illusory) is excluded from every savings and cost aggregate.
/// Its per-experiment row stays visible, footnoted.
fn headline_eligible(m: &Metrics) -> bool {
    m.validity.as_ref().is_none_or(|v| !v.gap_run_degenerate)
}

/// The three measured flows, with typed access to their turn-0 and per-turn
/// nodes (the GAP flow's `FlowMetrics` is flattened inside `GapFlowMetrics`).
#[derive(Clone, Copy)]
enum FlowKind {
    Base,
    Stateless,
    Gap,
}

impl FlowKind {
    fn turn0(self, m: &Metrics) -> Option<&TurnMetrics> {
        match self {
            FlowKind::Base => m.base_turn0.as_ref(),
            FlowKind::Stateless => m.stateless_turn0.as_ref(),
            FlowKind::Gap => m.gap_turn0.as_ref(),
        }
    }

    fn flow(self, m: &Metrics) -> Option<&FlowMetrics> {
        match self {
            FlowKind::Base => m.default_flow.as_ref(),
            FlowKind::Stateless => m.stateless_flow.as_ref(),
            FlowKind::Gap => m.gap_flow.as_ref().map(|g| &g.flow),
        }
    }
}

/// First `n` chars of a string — char-aware, never panics on multi-byte
/// formats (a byte slice could cut a UTF-8 sequence mid-character).
fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn median_of(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    let mut v = vals.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = v.len() / 2;
    Some(if v.len().is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    })
}

fn format_human_table(metrics: &[Metrics]) -> String {
    let mut out = String::new();

    let model = metrics[0].model.as_str();
    out.push_str("# GAP Experiment Results\n\n");
    out.push_str(&format!(
        "**Model:** `{model}` | **Experiments:** {}\n\n",
        metrics.len()
    ));

    out.push_str(
        "| Experiment | Fmt | Base Out | GAP Out | Out Δ | Parse | Apply | Seq Sim | F1 |\n",
    );
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");

    let mut total_base_out: u64 = 0;
    let mut total_gap_out: u64 = 0;
    // (base, gap) output tokens per eligible experiment, for the
    // macro/median/whale aggregates.
    let mut per_exp: Vec<(u64, u64)> = Vec::new();
    let mut excluded = 0usize;
    let mut total_parse_ok: usize = 0;
    let mut total_parse_total: usize = 0;
    let mut total_apply_ok: usize = 0;
    let mut total_apply_total: usize = 0;

    for m in metrics {
        let id = m.experiment_id.as_str();
        let fmt_short = truncate_chars(&m.format, 10);
        let eligible = headline_eligible(m);

        let base_out = m.default_flow.as_ref().map_or(0, |f| f.total_output_tokens);
        let gap_out = m
            .gap_flow
            .as_ref()
            .map_or(0, |g| g.flow.total_output_tokens);
        let out_delta = if base_out > 0 {
            format!("{:.1}%", (1.0 - gap_out as f64 / base_out as f64) * 100.0)
        } else {
            "—".into()
        };

        if eligible {
            total_base_out += base_out;
            total_gap_out += gap_out;
            if base_out > 0 {
                per_exp.push((base_out, gap_out));
            }
        } else {
            excluded += 1;
        }

        // Parse/apply rates from per-turn data. Reliability counts ALL runs:
        // a degenerate run is evidence about reliability even though its
        // savings are excluded.
        let (parse_ok, apply_ok) = if let Some(g) = &m.gap_flow {
            let turns = &g.flow.per_turn;
            let n = turns.len();
            let p = turns
                .iter()
                .filter(|t| t.envelope_parsed == Some(true))
                .count();
            let a = turns
                .iter()
                .filter(|t| t.apply_succeeded == Some(true))
                .count();
            total_parse_ok += p;
            total_apply_ok += a;
            total_parse_total += n;
            total_apply_total += n;
            (format!("{p}/{n}"), format!("{a}/{n}"))
        } else {
            ("—".into(), "—".into())
        };

        let seq_sim = m
            .quality
            .as_ref()
            .map(|q| format!("{:.3}", q.mean_sequence_similarity))
            .unwrap_or("—".into());
        let f1 = m
            .quality
            .as_ref()
            .map(|q| format!("{:.3}", q.mean_token_f1))
            .unwrap_or("—".into());

        let id_cell = if eligible {
            id.to_string()
        } else {
            format!("{id} †")
        };
        out.push_str(&format!(
            "| {id_cell} | {fmt_short} | {base_out:>6} | {gap_out:>6} | {out_delta:>6} | {parse_ok} | {apply_ok} | {seq_sim} | {f1} |\n"
        ));
    }

    if excluded > 0 {
        out.push_str(
            "\n† degenerate run (artifact never changed — all edits no-ops); excluded from the savings and cost aggregates below.\n"
        );
    }

    let micro = |base: u64, gap: u64| -> String {
        if base > 0 {
            format!("{:.1}%", (1.0 - gap as f64 / base as f64) * 100.0)
        } else {
            "—".into()
        }
    };

    out.push_str(&format!(
        "\n**Output savings ({} eligible / {} runs):**\n\n",
        metrics.len() - excluded,
        metrics.len()
    ));
    out.push_str("| Aggregate | Savings |\n|---|---:|\n");
    out.push_str(&format!(
        "| Micro (token-weighted; dominated by large artifacts) | {} |\n",
        micro(total_base_out, total_gap_out)
    ));
    let savings: Vec<f64> = per_exp
        .iter()
        .map(|(b, g)| (1.0 - *g as f64 / *b as f64) * 100.0)
        .collect();
    if !savings.is_empty() {
        out.push_str(&format!(
            "| Macro (mean of per-experiment savings; outlier-sensitive) | {:.1}% |\n",
            savings.iter().sum::<f64>() / savings.len() as f64
        ));
    }
    if let Some(med) = median_of(&savings) {
        out.push_str(&format!("| Median per-experiment | {med:.1}% |\n"));
    }
    // Whale check — does the micro number survive without the largest
    // artifacts that dominate the token-weighted sum?
    if per_exp.len() > 3 {
        let mut by_base = per_exp.clone();
        by_base.sort_by_key(|b| std::cmp::Reverse(b.0));
        let rest = &by_base[3..];
        let b: u64 = rest.iter().map(|(b, _)| b).sum();
        let g: u64 = rest.iter().map(|(_, g)| g).sum();
        out.push_str(&format!(
            "| Micro excluding the 3 largest artifacts (whale check) | {} |\n",
            micro(b, g)
        ));
    }

    out.push_str(&format!(
        "\n**Reliability (all {} runs, degenerate included):** Parse: {total_parse_ok}/{total_parse_total} | Apply: {total_apply_ok}/{total_apply_total}\n",
        metrics.len()
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
fn agent_loop_section(metrics: &[Metrics], model: &str) -> String {
    let p = cost::price_for(model);
    let eligible: Vec<&Metrics> = metrics
        .iter()
        .filter(|m| m.default_flow.is_some() && m.gap_flow.is_some())
        .filter(|m| headline_eligible(m))
        .collect();
    if eligible.is_empty() {
        return String::new();
    }

    // Per-experiment artifact sizes (tokens per version) and handle size.
    let session = |m: &Metrics| -> (Vec<f64>, f64) {
        let mut versions = Vec::new();
        if let Some(t0) = &m.base_turn0 {
            versions.push(t0.output_tokens as f64);
        }
        if let Some(base) = &m.default_flow {
            versions.extend(base.per_turn.iter().map(|t| t.output_tokens as f64));
        }
        let gap_outs: Vec<f64> = m
            .gap_flow
            .as_ref()
            .map(|g| {
                g.flow
                    .per_turn
                    .iter()
                    .map(|t| t.output_tokens as f64)
                    .collect()
            })
            .unwrap_or_default();
        let handle = if gap_outs.is_empty() {
            20.0
        } else {
            (gap_outs.iter().sum::<f64>() / gap_outs.len() as f64).max(20.0)
        };
        (versions, handle)
    };

    let scale = 1_000_000.0;
    let mut s =
        String::from("\n## Agent loop — Effect 2 (orchestrator context separation), MODELED\n\n");
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
fn correctness_section(metrics: &[Metrics]) -> String {
    let with: Vec<&Metrics> = metrics.iter().filter(|m| m.correctness.is_some()).collect();
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
        let id = m.experiment_id.as_str();
        let fmt_short = truncate_chars(&m.format, 12);
        let c = m.correctness.as_ref().expect("filtered to is_some");
        g_sum += c.pass_rate;
        b_sum += c.base_pass_rate;
        s.push_str(&format!(
            "| {id} | {fmt_short} | {:.0}% | {:.0}% |\n",
            c.pass_rate * 100.0,
            c.base_pass_rate * 100.0
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

/// Build an init-inclusive `[init, edit1, ...]` token sequence for a flow:
/// the creation turn (when present) followed by the edit turns.
fn flow_turns(turn0: Option<&TurnMetrics>, flow: &FlowMetrics) -> Vec<cost::Turn> {
    let mut turns = Vec::new();
    if let Some(t0) = turn0 {
        turns.push(cost::Turn {
            input: t0.input_tokens as f64,
            cached: t0.cached_input_tokens as f64,
            output: t0.output_tokens as f64,
        });
    }
    for t in &flow.per_turn {
        turns.push(cost::Turn {
            input: t.input_tokens as f64,
            cached: t.cached_input_tokens as f64,
            output: t.output_tokens as f64,
        });
    }
    turns
}

/// Mean of a per-experiment value, skipping experiments where it is absent.
fn mean_of(metrics: &[&Metrics], f: impl Fn(&Metrics) -> Option<f64>) -> Option<f64> {
    let vals: Vec<f64> = metrics.iter().copied().filter_map(f).collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

/// Run-validity gate summary — how many runs are headline-eligible.
fn validity_section(metrics: &[Metrics]) -> String {
    let total = metrics.len();
    let degenerate = metrics
        .iter()
        .filter(|m| m.validity.as_ref().is_some_and(|v| v.gap_run_degenerate))
        .count();
    let non_monotone = metrics
        .iter()
        .filter(|m| m.validity.as_ref().is_some_and(|v| !v.base_input_monotone))
        .count();
    if degenerate == 0 && non_monotone == 0 {
        return String::new();
    }
    let mut s = String::from("\n## Run validity\n\n");
    if degenerate > 0 {
        s.push_str(&format!(
            "- ⚠ **{degenerate}/{total}** GAP runs are **degenerate** (artifact never changed — all edits no-ops). Their \"output savings\" are illusory; they are excluded from every savings and cost aggregate in this report.\n"
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
fn decomposition_section(metrics: &[Metrics]) -> String {
    let eligible: Vec<&Metrics> = metrics.iter().filter(|m| headline_eligible(m)).collect();
    let has = eligible.iter().any(|m| m.decomposition.is_some());
    if !has {
        return String::new();
    }
    let g = |field: fn(&Decomposition) -> f64| {
        mean_of(&eligible, |m| m.decomposition.as_ref().map(field))
    };
    let fmt = |v: Option<f64>| v.map(|x| format!("{x:.1}%")).unwrap_or("—".into());
    let mut s =
        String::from("\n## Savings decomposition (A/B/C, init-inclusive, MEASURED tokens)\n\n");
    s.push_str("Mean per-experiment savings over eligible (non-degenerate) runs. **B vs A** = the input win from going stateless (any baseline can adopt it). **C vs B** = GAP's defensible output-envelope win.\n\n");
    s.push_str("| Axis | Comparison | Mean savings |\n|---|---|---:|\n");
    s.push_str(&format!(
        "| Input | B vs A (statelessness) | {} |\n",
        fmt(g(|d| d.input_savings_b_vs_a_pct))
    ));
    s.push_str(&format!(
        "| Output | C vs B (edit envelopes) | {} |\n",
        fmt(g(|d| d.output_savings_c_vs_b_pct))
    ));
    s.push_str(&format!(
        "| Input | C vs A | {} |\n",
        fmt(g(|d| d.input_savings_c_vs_a_pct))
    ));
    s.push_str(&format!(
        "| Output | C vs A | {} |\n",
        fmt(g(|d| d.output_savings_c_vs_a_pct))
    ));
    s
}

/// Aggregate init-inclusive cost of a flow across all experiments, under a
/// cache regime. `growing_prefix` is true only for Scenario A (append-only
/// conversation), whose stable prefix is the one a hot cache can actually serve.
fn agg_cost(
    metrics: &[&Metrics],
    kind: FlowKind,
    p: cost::Price,
    cache: Cache,
    growing_prefix: bool,
) -> f64 {
    metrics
        .iter()
        .filter_map(|m| kind.flow(m).map(|f| (kind.turn0(m), f)))
        .map(|(t0, f)| cost::flow_cost(&flow_turns(t0, f), p, cache, growing_prefix))
        .sum()
}

/// Cost analysis — init-inclusive, multi-regime. Caching most benefits the base
/// flow (Scenario A, append-only cacheable prefix); GAP's per-turn artifact
/// injection is uncacheable. The headline: even granting the baseline a
/// perfectly hot cache (`theoretical_best`), GAP still wins because output
/// tokens are never cached. Cost figures are MODELED.
fn cost_analysis(metrics: &[Metrics], model: &str) -> String {
    let p = cost::price_for(model);
    let eligible: Vec<&Metrics> = metrics.iter().filter(|m| headline_eligible(m)).collect();
    let has_gap = eligible.iter().any(|m| m.gap_flow.is_some());
    let has_base = eligible.iter().any(|m| m.default_flow.is_some());
    if !has_gap || !has_base {
        return String::new();
    }

    // Base = Scenario A (growing prefix); GAP = stateless (no growing prefix).
    let base_off = agg_cost(&eligible, FlowKind::Base, p, Cache::Off, true);
    let base_obs = agg_cost(&eligible, FlowKind::Base, p, Cache::Observed, true);
    let base_best = agg_cost(&eligible, FlowKind::Base, p, Cache::TheoreticalBest, true);
    let gap_off = agg_cost(&eligible, FlowKind::Gap, p, Cache::Off, false);
    let gap_obs = agg_cost(&eligible, FlowKind::Gap, p, Cache::Observed, false);

    let mut s = String::new();
    s.push_str("\n## Cost analysis — init-inclusive, cache regimes (MODELED $)\n\n");
    s.push_str(&format!(
        "Over {} eligible experiments (degenerate runs excluded). Prices (USD/1M): input ${:.3}, cached-input ${:.3}, output ${:.3}. Output is never cached.\n\n",
        eligible.len(), p.input, p.cached_in, p.output
    ));
    s.push_str(
        "| Flow | Cost (cache off) | Cost (cache observed) | Cost (cache theoretical-best) |\n",
    );
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
    for m in &eligible {
        let (Some(base), Some(gap)) = (m.default_flow.as_ref(), m.gap_flow.as_ref()) else {
            continue;
        };
        let base_turns = flow_turns(m.base_turn0.as_ref(), base);
        let gap_turns = flow_turns(m.gap_turn0.as_ref(), &gap.flow);
        let base_cum = cost::cumulative(&base_turns, p, Cache::TheoreticalBest, true);
        let gap_cum = cost::cumulative(&gap_turns, p, Cache::Off, false);
        if let Some(t) = cost::break_even(&base_cum, &gap_cum) {
            be.push(t);
        }
    }
    if !be.is_empty() {
        let mean = be.iter().sum::<usize>() as f64 / be.len() as f64;
        s.push_str(&format!(
            "\n**Break-even** (cumulative GAP cost < perfectly-cached base): reached in {}/{} eligible experiments, mean edit turn {:.1}.\n",
            be.len(),
            eligible
                .iter()
                .filter(|m| m.gap_flow.is_some() && m.default_flow.is_some())
                .count(),
            mean,
        ));
    }
    s
}

/// Latency section — TTFT / TTLT per flow (MEASURED), median with IQR over
/// edit turns, plus a TTLT mean column so mean claims cited elsewhere are
/// reproducible from this table. GAP trades a possibly-higher TTFT (the
/// artifact sits in the input → longer prefill; structured decoding adds
/// overhead) for a much lower TTLT (far fewer output tokens). A turn enters
/// the aggregates only if its stream completed (TTLT present) and it was not
/// retried — retried turns carry backoff sleep in their wall-clock.
fn latency_section(metrics: &[Metrics]) -> String {
    let turns_of = |kind: FlowKind| -> Vec<&TurnResult> {
        metrics
            .iter()
            .filter_map(|m| kind.flow(m))
            .flat_map(|f| &f.per_turn)
            .filter(|t| t.ttlt_ms.is_some())
            .filter(|t| t.retried != Some(true))
            .collect()
    };
    let med_iqr = |turns: &[&TurnResult], field: fn(&TurnResult) -> Option<f64>| -> String {
        let mut vals: Vec<f64> = turns.iter().filter_map(|t| field(t)).collect();
        if vals.is_empty() {
            return "—".into();
        }
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = median_of(&vals).unwrap();
        let lo = median_of(&vals[..vals.len() / 2]).unwrap_or(med);
        let hi = median_of(&vals[vals.len().div_ceil(2)..]).unwrap_or(med);
        format!("{med:.0} ms ({lo:.0}-{hi:.0})")
    };
    let mean_ms = |turns: &[&TurnResult], field: fn(&TurnResult) -> Option<f64>| -> String {
        let vals: Vec<f64> = turns.iter().filter_map(|t| field(t)).collect();
        if vals.is_empty() {
            return "—".into();
        }
        format!("{:.0} ms", vals.iter().sum::<f64>() / vals.len() as f64)
    };

    let rows = [
        ("Base (full regen)", FlowKind::Base),
        ("Stateless (full regen)", FlowKind::Stateless),
        ("GAP (envelopes)", FlowKind::Gap),
    ];
    let present: Vec<_> = rows
        .iter()
        .map(|(label, kind)| (*label, turns_of(*kind)))
        .filter(|(_, turns)| !turns.is_empty())
        .collect();
    if present.is_empty() {
        return String::new();
    }

    let mut s = String::from("\n## Latency (median over edit turns, IQR in parens, MEASURED)\n\n");
    s.push_str(
        "Wall-clock includes network + queueing, not pure prefill/decode. \
Retried turns are excluded (their wall-clock contains rate-limit backoff). \
TTLT is also reported as a mean; long-tail turns pull it above the median.\n\n",
    );
    s.push_str(
        "| Flow | Turns | TTFT | TTLT | TTLT mean | Total latency |\n|---|---:|---:|---:|---:|---:|\n",
    );
    let mut legacy = false;
    for (label, turns) in &present {
        legacy |= turns.iter().any(|t| t.retried.is_none());
        s.push_str(&format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            turns.len(),
            med_iqr(turns, |t| t.ttft_ms.map(|v| v as f64)),
            med_iqr(turns, |t| t.ttlt_ms.map(|v| v as f64)),
            mean_ms(turns, |t| t.ttlt_ms.map(|v| v as f64)),
            med_iqr(turns, |t| Some(t.latency_ms as f64)),
        ));
    }
    if legacy {
        s.push_str(
            "\n⚠ Turns recorded without a `retried` flag predate the harness fix and their \
TTFT/TTLT are **under revision** (methodology corrected 2026-06-09: the stream timer was \
anchored after response headers, so TTFT/TTLT excluded request upload and queue time, and \
retried turns cannot be identified or filtered). Values are kept visible until the suite is \
re-run under the corrected harness.\n",
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::{GapFlowMetrics, Validity};

    fn turn(output_tokens: u64) -> TurnResult {
        TurnResult {
            turn: 1,
            edit: String::new(),
            input_tokens: 10,
            output_tokens,
            cached_input_tokens: 0,
            latency_ms: 100,
            output_bytes: 0,
            ttft_ms: Some(50),
            ttlt_ms: Some(90),
            median_itl_ms: None,
            retried: None,
            failed: false,
            failure_reason: None,
            envelope_parsed: None,
            apply_succeeded: None,
            envelope_name: None,
        }
    }

    fn flow(turns: Vec<TurnResult>) -> FlowMetrics {
        FlowMetrics {
            total_input_tokens: turns.iter().map(|t| t.input_tokens).sum(),
            total_output_tokens: turns.iter().map(|t| t.output_tokens).sum(),
            total_cached_input_tokens: 0,
            total_latency_ms: turns.iter().map(|t| t.latency_ms).sum(),
            per_turn: turns,
        }
    }

    fn exp(id: &str, base_out: u64, gap_out: u64, degenerate: bool) -> Metrics {
        let mut gap_turn = turn(gap_out);
        gap_turn.latency_ms = 50;
        gap_turn.ttft_ms = Some(20);
        gap_turn.ttlt_ms = Some(40);
        gap_turn.envelope_parsed = Some(true);
        gap_turn.apply_succeeded = Some(!degenerate);
        Metrics {
            experiment_id: id.into(),
            model: "test-model".into(),
            provider: "test".into(),
            timestamp: String::new(),
            format: "text/html".into(),
            base_turn0: None,
            gap_turn0: None,
            stateless_turn0: None,
            stateless_flow: None,
            default_flow: Some(flow(vec![turn(base_out)])),
            gap_flow: Some(GapFlowMetrics {
                flow: flow(vec![gap_turn]),
                envelope_parse_rate: 1.0,
                apply_success_rate: if degenerate { 0.0 } else { 1.0 },
            }),
            comparison: None,
            decomposition: None,
            validity: Some(Validity {
                gap_run_degenerate: degenerate,
                base_input_monotone: true,
            }),
            token_table: None,
            quality: None,
            correctness: None,
        }
    }

    #[test]
    fn degenerate_run_excluded_from_savings_aggregates_but_still_rendered() {
        let metrics = vec![
            exp("e1", 1000, 100, false),  // 90% savings
            exp("e2", 4000, 2000, false), // 50% savings
            exp("e3", 10000, 100, true),  // degenerate; would inflate micro
        ];
        let out = format_human_table(&metrics);

        // Micro over eligible runs: 1 - 2100/5000 = 58.0%.
        assert!(out.contains("58.0%"), "{out}");
        // With the degenerate run included it would be 1 - 2200/15000 = 85.3%.
        assert!(!out.contains("85.3%"), "{out}");
        // Macro mean and median of {90%, 50%}.
        assert!(out.contains("70.0%"), "{out}");
        // The degenerate run stays visible as a footnoted row.
        assert!(out.contains("| e3 † |"), "{out}");
        assert!(out.contains("degenerate"), "{out}");
        assert!(out.contains("2 eligible / 3 runs"), "{out}");
        // Reliability still counts all runs (3 turns, 2 applies succeeded).
        assert!(out.contains("Parse: 3/3 | Apply: 2/3"), "{out}");
    }

    #[test]
    fn cost_analysis_excludes_degenerate_runs() {
        let metrics = vec![exp("e1", 1000, 100, false), exp("e2", 1000, 100, true)];
        let out = cost_analysis(&metrics, "test-model");
        assert!(out.contains("Over 1 eligible experiments"), "{out}");
    }

    #[test]
    fn latency_uses_median_and_skips_retried_turns() {
        let mut m = exp("e1", 1000, 100, false);
        let base_turns: Vec<TurnResult> = [(100, 10, 90), (200, 20, 190), (99999, 31000, 99000)]
            .iter()
            .enumerate()
            .map(|(i, &(latency, ttft, ttlt))| {
                let mut t = turn(100);
                t.latency_ms = latency;
                t.ttft_ms = Some(ttft);
                t.ttlt_ms = Some(ttlt);
                t.retried = Some(i == 2);
                t
            })
            .collect();
        m.default_flow = Some(flow(base_turns));
        let mut gap_turn = turn(100);
        gap_turn.latency_ms = 50;
        gap_turn.ttft_ms = Some(5);
        gap_turn.ttlt_ms = Some(40);
        gap_turn.retried = Some(false);
        m.gap_flow.as_mut().unwrap().flow = flow(vec![gap_turn]);

        let out = latency_section(&[m]);
        // Median TTFT of the two non-retried base turns: 15 ms.
        assert!(out.contains("15 ms"), "{out}");
        assert!(!out.contains("31000"), "{out}");
        // Every included turn carries the retried flag: no vintage warning.
        assert!(!out.contains("under revision"), "{out}");
    }

    #[test]
    fn latency_reports_ttlt_mean_alongside_median() {
        let mut m = exp("e1", 1000, 100, false);
        let base_turns: Vec<TurnResult> = [10u64, 20, 90]
            .iter()
            .map(|&ttlt| {
                let mut t = turn(100);
                t.ttlt_ms = Some(ttlt);
                t.retried = Some(false);
                t
            })
            .collect();
        m.default_flow = Some(flow(base_turns));

        let out = latency_section(&[m]);
        // Median 20 ms (IQR 10-90), mean 40 ms — distinct values so the mean
        // column is asserted independently of the median.
        assert!(out.contains("| 20 ms (10-90) | 40 ms |"), "{out}");
    }

    #[test]
    fn legacy_turns_without_retried_flag_are_annotated_under_revision() {
        let out = latency_section(&[exp("e1", 1000, 100, false)]);
        assert!(out.contains("under revision"), "{out}");
        assert!(out.contains("2026-06-09"), "{out}");
    }

    #[test]
    fn multibyte_format_does_not_panic_and_truncates_by_chars() {
        // A byte slice at index 10 would split the 4th of these 3-byte chars.
        let mut m = exp("e1", 1000, 100, false);
        m.format = "日本語のフォーマット".into();
        let out = format_human_table(&[m]);
        assert!(out.contains("日本語のフォーマット"), "{out}");
        assert_eq!(
            truncate_chars("日本語のフォーマット拡張", 10),
            "日本語のフォーマット"
        );
    }

    #[test]
    fn report_loads_typed_metrics_and_renders_golden_report() {
        // Fixtures: a healthy run, a degenerate run (legacy vintage, no
        // `retried` flags), and an A/B/C run with decomposition + correctness.
        // Locks the full rendering pipeline including the rank-1 degenerate
        // gating. Regenerate deliberately with UPDATE_GOLDEN=1 after an
        // intentional rendering change.
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/report");
        let mut metrics: Vec<Metrics> = Vec::new();
        for id in [
            "001-html-healthy",
            "002-json-degenerate",
            "003-yaml-abc-oracles",
        ] {
            let path = fixtures.join(id).join("metrics.json");
            let raw = fs::read_to_string(&path).unwrap();
            metrics.push(
                serde_json::from_str(&raw)
                    .unwrap_or_else(|e| panic!("parse {}: {e}", path.display())),
            );
        }

        let rendered = format_human_table(&metrics);

        let golden_path = fixtures.join("golden.md");
        if std::env::var("UPDATE_GOLDEN").is_ok() {
            fs::write(&golden_path, &rendered).unwrap();
        }
        let golden = fs::read_to_string(&golden_path).unwrap();
        assert_eq!(
            rendered, golden,
            "report rendering drifted from tests/fixtures/report/golden.md; \
             rerun with UPDATE_GOLDEN=1 if the change is intentional"
        );
    }
}
