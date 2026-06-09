//! Unified cost accounting — one source of truth for every dollar figure.
//!
//! All cost math lives here (not scattered in `report.rs`) so cache discounts,
//! init/turn-0 cost, and the A/B/C decomposition are computed the same way
//! everywhere and nothing is double-counted. The functions are pure and
//! unit-tested.
//!
//! ## Cache model
//!
//! These endpoints speak the OpenAI wire format, where prompt caching is
//! **automatic** and carries **no write surcharge**: cache *hits* are billed at
//! a discount (`cached_in`), cache *misses* at the full `input` rate and are
//! written to the cache for free. So a turn's cost is simply
//!
//! ```text
//! cost = (input - cached)·p_in + cached·p_cached_in + output·p_out
//! ```
//!
//! and there is no separate write line (Anthropic-style 1.25× writes are out of
//! scope; noted in STEELMAN.md). The only question is *how many* input tokens
//! are cached under each [`Cache`] regime — see [`cached_for`].

/// Per-1M-token USD prices. `cached_in` is the discounted rate for input tokens
/// served from the provider's prompt cache. Output is never cached.
#[derive(Clone, Copy, Debug)]
pub struct Price {
    pub input: f64,
    pub cached_in: f64,
    pub output: f64,
}

/// Price table (USD / 1M tokens). Adjust here if rates change.
pub fn price_for(model: &str) -> Price {
    let m = model.to_lowercase();
    // gpt-5 family: cached input billed at 10% of input (90% discount).
    if m.contains("gpt-5.4-mini") || m.contains("gpt-5-mini") {
        Price {
            input: 0.25,
            cached_in: 0.025,
            output: 2.00,
        }
    } else if m.contains("gpt-5.4-nano") || m.contains("gpt-5-nano") {
        Price {
            input: 0.05,
            cached_in: 0.005,
            output: 0.40,
        }
    } else if m.contains("gpt-5") {
        Price {
            input: 1.25,
            cached_in: 0.125,
            output: 10.00,
        }
    } else if m.contains("gpt-4o-mini") {
        Price {
            input: 0.15,
            cached_in: 0.075,
            output: 0.60,
        }
    } else if m.contains("gpt-4o") {
        Price {
            input: 2.50,
            cached_in: 1.25,
            output: 10.00,
        }
    } else {
        // Unknown model: assume input=cached (no caching benefit) and 4x output.
        Price {
            input: 1.0,
            cached_in: 1.0,
            output: 4.0,
        }
    }
}

/// One turn's observed token counts. Index 0 of a `&[Turn]` slice is the
/// init/creation turn; the rest are edit turns. `cached` is the provider-
/// reported cache hit (used only by [`Cache::Observed`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Turn {
    pub input: f64,
    pub cached: f64,
    pub output: f64,
}

/// Cache billing regime applied to a whole flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cache {
    /// No caching: every input token billed at full rate.
    Off,
    /// Use the provider-reported `cached` count verbatim (may be 0 on free
    /// tiers that don't report `prompt_tokens_details`).
    Observed,
    /// Steelman the baseline: assume a perfectly hot cache. For a flow with a
    /// `growing_prefix` (Scenario A — append-only conversation), every token
    /// carried over from the previous turn is a cache hit, so the only fresh
    /// input each turn is the new edit message. For stateless flows (B, C) the
    /// per-turn artifact injection changes every turn and is *not* cacheable, so
    /// this falls back to the observed hit (only the stable system prompt would
    /// cache, which is small and conservatively ignored here — it can only help
    /// GAP's opponent less, never more).
    TheoreticalBest,
}

/// How many of turn `k`'s input tokens are billed at the cached rate, given the
/// previous turn (`prev`) and whether this flow accumulates a growing prefix.
pub fn cached_for(t: Turn, prev: Option<Turn>, cache: Cache, growing_prefix: bool) -> f64 {
    match cache {
        Cache::Off => 0.0,
        Cache::Observed => t.cached.min(t.input),
        Cache::TheoreticalBest => {
            if growing_prefix {
                // Append-only: turn k's message list contains all of turn k-1's
                // input plus last turn's assistant output, then the new edit.
                // Everything but the new edit message is a cache hit.
                match prev {
                    Some(p) => (p.input + p.output).min(t.input),
                    None => 0.0, // first turn: nothing to cache yet
                }
            } else {
                // Stateless flows re-inject a changed artifact each turn; the
                // bulk is uncacheable. Be conservative — credit only what the
                // provider actually cached.
                t.cached.min(t.input)
            }
        }
    }
}

/// Cost of a single turn (USD) under a regime.
pub fn turn_cost(t: Turn, prev: Option<Turn>, p: Price, cache: Cache, growing_prefix: bool) -> f64 {
    const SCALE: f64 = 1_000_000.0;
    let cached = cached_for(t, prev, cache, growing_prefix);
    let uncached = (t.input - cached).max(0.0);
    (uncached * p.input + cached * p.cached_in + t.output * p.output) / SCALE
}

/// Total cost across every turn (USD). **Init-inclusive**: pass the full
/// sequence `[init, edit1, edit2, ...]`.
pub fn flow_cost(turns: &[Turn], p: Price, cache: Cache, growing_prefix: bool) -> f64 {
    cumulative(turns, p, cache, growing_prefix)
        .last()
        .copied()
        .unwrap_or(0.0)
}

/// Cumulative cost after each turn (USD), init-inclusive.
pub fn cumulative(turns: &[Turn], p: Price, cache: Cache, growing_prefix: bool) -> Vec<f64> {
    let mut out = Vec::with_capacity(turns.len());
    let mut acc = 0.0;
    let mut prev: Option<Turn> = None;
    for &t in turns {
        acc += turn_cost(t, prev, p, cache, growing_prefix);
        out.push(acc);
        prev = Some(t);
    }
    out
}

/// First index at which `gap` cumulative cost drops below `base` cumulative
/// cost. Both slices must be aligned turn-for-turn (index 0 = init). Returns the
/// **edit turn number** (1-based, init excluded), or `None` if GAP never wins
/// within the measured window.
pub fn break_even(base: &[f64], gap: &[f64]) -> Option<usize> {
    base.iter()
        .zip(gap.iter())
        .enumerate()
        .find(|(_, (b, g))| g < b)
        .map(|(i, _)| i) // index 0 is init; index 1 is the first edit
        .filter(|&i| i >= 1)
}

/// Savings percentage of `gap` relative to `base` (positive = GAP cheaper).
pub fn savings_pct(base: f64, gap: f64) -> f64 {
    if base > 0.0 {
        (1.0 - gap / base) * 100.0
    } else {
        0.0
    }
}

/// Effect-2 (orchestrator context separation) projection — the tool-use win.
///
/// In a tool-using agent, the artifact-producing tool returns its result into
/// the conversation, and the orchestrator re-reads its context on every turn.
/// This models the **orchestrator's** cumulative input tokens over a session of
/// `versions.len() + extra_turns` turns under three regimes. MODELED from the
/// measured artifact sizes (`versions` — tokens per version, e.g. the base
/// flow's full-regen output per turn) and a conservative `handle` size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AgentLoop {
    pub turns: usize,
    /// Worst-case baseline: every version's full body stays in the transcript
    /// forever (quadratic). Reported only as a labeled worst case.
    pub accumulate_input: f64,
    /// Steelman baseline: only the current body is kept in context, re-read
    /// each turn (linear). What a competent operator deploys.
    pub keep_latest_input: f64,
    /// GAP: the orchestrator holds only a handle; the body never enters its
    /// context (editing happens in the separate maintain wallet).
    pub gap_input: f64,
    /// Full-body re-reads GAP avoids vs KeepLatest (one per turn).
    pub rereads_avoided: usize,
}

/// `versions` are artifact sizes in tokens at each version (index 0 = creation).
/// `handle` is a conservative handle size in tokens. `extra_turns` are the
/// reasoning turns the orchestrator spends on the *final* artifact after the
/// last edit — the regime divergence widens with each one.
pub fn agent_loop(versions: &[f64], handle: f64, extra_turns: usize) -> AgentLoop {
    let n = versions.len();
    if n == 0 {
        return AgentLoop::default();
    }
    let last = versions[n - 1];
    let total: f64 = versions.iter().sum();
    let turns = n + extra_turns;

    // KeepLatest: each version is read on its turn; the tail re-reads the final.
    let keep_latest_input = total + extra_turns as f64 * last;

    // Accumulate: turn j re-reads every body produced through turn j (quadratic).
    let mut prefix = 0.0;
    let mut accumulate_input = 0.0;
    for &v in versions {
        prefix += v;
        accumulate_input += prefix;
    }
    accumulate_input += extra_turns as f64 * total;

    // GAP: a single handle, re-read each turn.
    let gap_input = turns as f64 * handle;

    AgentLoop {
        turns,
        accumulate_input,
        keep_latest_input,
        gap_input,
        rereads_avoided: turns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Price = Price {
        input: 3.0,
        cached_in: 0.3,
        output: 15.0,
    };

    fn t(input: f64, output: f64) -> Turn {
        Turn {
            input,
            cached: 0.0,
            output,
        }
    }

    #[test]
    fn off_regime_ignores_cache() {
        let turn = Turn {
            input: 1000.0,
            cached: 800.0,
            output: 100.0,
        };
        // cache Off: all 1000 input at full rate + output.
        let expect = (1000.0 * 3.0 + 100.0 * 15.0) / 1e6;
        assert!((turn_cost(turn, None, P, Cache::Off, false) - expect).abs() < 1e-12);
    }

    #[test]
    fn observed_regime_uses_reported_cache() {
        let turn = Turn {
            input: 1000.0,
            cached: 800.0,
            output: 100.0,
        };
        let expect = (200.0 * 3.0 + 800.0 * 0.3 + 100.0 * 15.0) / 1e6;
        assert!((turn_cost(turn, None, P, Cache::Observed, false) - expect).abs() < 1e-12);
    }

    #[test]
    fn theoretical_best_caches_growing_prefix() {
        // Append-only conversation: turn 0 then turn 1.
        let t0 = t(500.0, 2000.0); // init
        let t1 = t(2600.0, 2000.0); // re-reads system+artifact+asst, +small edit
                                    // turn 1 cached = t0.input + t0.output = 2500; fresh = 100.
        let prev = Some(t0);
        let cached = cached_for(t1, prev, Cache::TheoreticalBest, true);
        assert_eq!(cached, 2500.0);
        let expect = (100.0 * 3.0 + 2500.0 * 0.3 + 2000.0 * 15.0) / 1e6;
        assert!((turn_cost(t1, prev, P, Cache::TheoreticalBest, true) - expect).abs() < 1e-12);
    }

    #[test]
    fn theoretical_best_stateless_falls_back_to_observed() {
        let prev = Some(t(500.0, 2000.0));
        let turn = Turn {
            input: 2600.0,
            cached: 0.0,
            output: 30.0,
        };
        // not growing -> cached stays at observed (0).
        let cached = cached_for(turn, prev, Cache::TheoreticalBest, false);
        assert_eq!(cached, 0.0);
    }

    #[test]
    fn break_even_after_one_edit() {
        // GAP cheaper from the first edit onward.
        let base = vec![1.0, 2.0, 3.0, 4.0];
        let gap = vec![1.0, 1.5, 1.6, 1.7];
        assert_eq!(break_even(&base, &gap), Some(1));
    }

    #[test]
    fn break_even_never_when_gap_loses() {
        let base = vec![1.0, 2.0, 3.0];
        let gap = vec![1.0, 2.5, 3.5];
        assert_eq!(break_even(&base, &gap), None);
    }

    #[test]
    fn cumulative_is_init_inclusive() {
        let turns = vec![t(500.0, 2000.0), t(2000.0, 30.0)];
        let cum = cumulative(&turns, P, Cache::Off, false);
        let c0 = (500.0 * 3.0 + 2000.0 * 15.0) / 1e6;
        let c1 = c0 + (2000.0 * 3.0 + 30.0 * 15.0) / 1e6;
        assert!((cum[0] - c0).abs() < 1e-12);
        assert!((cum[1] - c1).abs() < 1e-12);
    }

    #[test]
    fn gap_output_win_survives_baseline_caching() {
        // The steelman headline: even with the baseline perfectly cached, GAP
        // still wins because output is never cached. Index 0 is the shared init
        // (a wash). Base = Scenario A (growing prefix), GAP = stateless edits.
        let base = vec![t(500.0, 2000.0), t(2600.0, 2000.0), t(4700.0, 2000.0)];
        let gap = vec![t(500.0, 2000.0), t(2500.0, 30.0), t(2530.0, 30.0)];
        let base_cost = flow_cost(&base, P, Cache::TheoreticalBest, true);
        let gap_cost = flow_cost(&gap, P, Cache::Off, false);
        assert!(
            gap_cost < base_cost,
            "GAP must be cheaper than a perfectly-cached baseline"
        );

        // The un-erasable mechanism: the baseline's cost is dominated by output
        // tokens, which no cache can discount. Sum output cost across both flows.
        const SCALE: f64 = 1_000_000.0;
        let out_cost = |ts: &[Turn]| ts.iter().map(|t| t.output * P.output / SCALE).sum::<f64>();
        // With a perfectly hot cache the baseline's bill is mostly output...
        assert!(
            out_cost(&base) / base_cost > 0.5,
            "perfectly-cached baseline cost should be output-dominated (got {:.0}%)",
            out_cost(&base) / base_cost * 100.0
        );
        // ...and GAP slashes exactly that term on the edit turns (init is a wash).
        let base_edit_out = out_cost(&base[1..]);
        let gap_edit_out = out_cost(&gap[1..]);
        assert!(
            savings_pct(base_edit_out, gap_edit_out) > 95.0,
            "GAP edit-turn output-cost savings should exceed 95% (got {:.1}%)",
            savings_pct(base_edit_out, gap_edit_out)
        );
    }

    #[test]
    fn savings_grow_as_init_amortizes() {
        // Init is a one-time GAP cost; the longer the artifact lives, the more
        // the per-edit output win dominates. Full-lifecycle savings must rise
        // with more edits.
        let base: Vec<Turn> = std::iter::once(t(500.0, 2000.0))
            .chain((1..=10).map(|k| t(500.0 + 2100.0 * k as f64, 2000.0)))
            .collect();
        let gap: Vec<Turn> = std::iter::once(t(500.0, 2000.0))
            .chain((1..=10).map(|_| t(2500.0, 30.0)))
            .collect();
        let s2 = savings_pct(
            flow_cost(&base[..3], P, Cache::TheoreticalBest, true),
            flow_cost(&gap[..3], P, Cache::Off, false),
        );
        let s10 = savings_pct(
            flow_cost(&base, P, Cache::TheoreticalBest, true),
            flow_cost(&gap, P, Cache::Off, false),
        );
        assert!(
            s10 > s2,
            "10-edit savings ({s10:.1}%) should exceed 2-edit savings ({s2:.1}%)"
        );
    }

    #[test]
    fn agent_loop_orders_regimes() {
        // handle (20) << any artifact version: gap < keep_latest < accumulate.
        let versions = vec![2000.0, 2000.0, 2000.0, 2000.0];
        let al = agent_loop(&versions, 20.0, 5);
        assert!(al.gap_input < al.keep_latest_input);
        assert!(al.keep_latest_input < al.accumulate_input);
        assert_eq!(al.turns, 9);
        assert_eq!(al.rereads_avoided, 9);
    }

    #[test]
    fn agent_loop_accumulate_diverges_superlinearly() {
        // Doubling the tail more-than-doubles the Accumulate orchestrator cost
        // (quadratic), while GAP stays linear in turns.
        let versions = vec![2000.0; 6];
        let a5 = agent_loop(&versions, 20.0, 5);
        let a10 = agent_loop(&versions, 20.0, 10);
        let acc_growth = a10.accumulate_input / a5.accumulate_input;
        let gap_growth = a10.gap_input / a5.gap_input;
        assert!(
            acc_growth > gap_growth,
            "Accumulate should grow faster than GAP (acc {acc_growth:.2}× vs gap {gap_growth:.2}×)"
        );
    }

    #[test]
    fn agent_loop_gap_saves_vs_keep_latest() {
        // Even vs the steelman KeepLatest baseline, holding handles instead of
        // the body slashes orchestrator input.
        let versions = vec![2000.0, 2050.0, 2100.0];
        let al = agent_loop(&versions, 25.0, 10);
        assert!(savings_pct(al.keep_latest_input, al.gap_input) > 95.0);
    }

    #[test]
    fn agent_loop_empty_is_zero() {
        assert_eq!(agent_loop(&[], 20.0, 5), AgentLoop::default());
    }
}
