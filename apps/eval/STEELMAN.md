# STEELMAN.md — A Defensible GAP Benchmark

> Design doc for upgrading `gap-eval` so its numbers survive a hostile reviewer.
> Companion to [`assets/evals/experiments/EXPERIMENT.md`](../../assets/evals/experiments/EXPERIMENT.md)
> (which it supersedes on methodology). Status: proposed. Drafted 2026-05-28.

## 0. Verified evidence this is needed (committed run `026-json-api-response-users`, git HEAD)

```
BASE (turn, in,  out,  bytes):  (1, 4082, 517, 1530)  (2,  624, 1562, 4689)  (3, 2230, 1955, 5641)
GAP  (turn, in,  out,  bytes):  (1, 4042, 721, 8661)  (2, 4040,  191, 8661)  (3, 4044,  272, 8661)
gap apply_success_rate: 0   parse_rate: 0.667
comparison: output_savings 70.6%   input_savings -74.8%   latency_savings 47.6%
```

Three independently-disqualifying facts in one committed example:

1. **The 70.6% "output savings" is a lie.** GAP's `output_bytes` is frozen at 8661 across all three turns and `apply_success_rate = 0` — every edit failed, the artifact never changed. The "savings" is the cost of three failed envelope attempts on a static document. The eval scores this as a win.
2. **The baseline input is cache-corrupted.** BASE input 4082 → 624 → 2230 is non-monotone, which is physically impossible for an append-only growing conversation. The provider (`ollama`/`gemma4`) reports **post-cache** `prompt_tokens`. Any input-axis claim is uninterpretable until this is gated.
3. **GAP loses on input here (−74.8%).** On a small artifact, re-reading it every stateless turn costs *more* than the (cached) baseline. This is a real, expected loss region the eval currently buries inside an aggregate.

---

## 1. Executive summary — the steelman thesis

GAP has exactly **one win that no baseline optimization can erase: output-token reduction** — the LLM emits a `d_k`-token edit envelope instead of regenerating the full `S_k`-token artifact, and `d_k` is consumed by the apply engine, never re-read. Output tokens are never cached and cost `r = p_out/p_in` (1×–5×) more than input. Everything else GAP appears to win — going stateless, flattening context, separating the orchestrator from the artifact body — is a *technique available to the baseline too* (Scenario B), so it must be granted to the baseline before any win is claimed.

The steelman discipline is therefore: **give the baseline its strongest honest form** (stateless, optimally and *symmetrically* cached, same model, same structured-output tax), show that GAP *still* wins, and **say precisely where it does not**. The second, larger but harder-to-measure win is **Effect 2 (orchestrator context separation)** in agent loops — but only against a *competent* baseline (artifact stored out-of-band behind a reference, re-read on demand), never against the strawman that re-injects the full body every step.

The current eval cannot prove any of this because it: (a) runs only Scenario A and C, conflating the stateless-input win with the envelope-output win; (b) generates two separate turn-0 artifacts (`runner.rs:84` vs `runner.rs:166`) so base and GAP edit different documents; (c) excludes init/turn-0 from all cost totals (`report.rs:204` "edit turns only"), hiding GAP's protocol overhead; (d) collects TTFT/TTLT/ITL but never reports them and anchors `t0` after response headers (`client.rs:173`); (e) scores GAP output against BASE output (`scorer.rs:216`) — a category error; (f) runs once with no variance; (g) never models the agent loop where Effect 2 lives; and (h) lets the degenerate `026` run above report 70.6% "savings".

The single biggest correctness risk threading every dimension is **double-counting in cost**: cache writes, init turns, the maintain-context wallet, and failed-envelope fallback all touch cost and must live in **one `cost.rs` module** so nothing is billed twice or silently omitted.

---

## 2. What the current eval cannot answer (gap table)

| Question the steelman must answer | Why it can't today | Verified in code |
|---|---|---|
| How much saving is *input* (statelessness) vs *output* (envelopes)? | Only A and C run; B missing | `experiment.rs:298-299` |
| Do base and GAP edit the *same* document? | Two turn-0 generations | `runner.rs:84`, `runner.rs:166` (026: 10241B vs 8661B) |
| What is GAP's protocol overhead / break-even turn? | Init excluded from all totals | `report.rs:204` "edit turns only" |
| Does caching the baseline erase GAP's edge? | Cache modeled only from observed `cached_tokens`, asymmetric, edit-turns only | `report.rs:166-201`; free tiers return `None` |
| What are TTFT / decode / ITL per flow? | Collected, never reported; `t0` after headers | `client.rs:173`, `:226-242` |
| Is the GAP *correct* (intent/validity), not just cheaper? | Scores GAP-vs-BASE similarity, not ground truth | `scorer.rs:216` |
| Is a result stable or noise? | Single run; `seed=42` but gpt-5 forces `temperature=None`→1.0 | `client.rs:79,86` |
| What does GAP save in an agent loop (Effect 2)? | No agent-loop flow exists | absent |
| What does GAP cost when an edit *fails*? | Failures still counted as savings | 026: `apply=0` → 70.6% "savings" |
| When is GAP **not** worth it? | No d/S, no break-even, no loss region | absent |

---

## 3. The measurement model

### 3.1 Scenarios (spec `gap.md:444-476`)

- **A — naive conversation:** one growing context, full regen. Input at edit `k` = `I + S₀+…+S_{k-1}` (O(N²S)). The *weakest* baseline; used only to show the cost of *not* going stateless. **Never the headline comparator.**
- **B — stateless full regen (THE STEELMAN BASELINE):** fresh 2-message context per edit, reads `I + S_{k-1}`, regenerates full `S_k`. Input linear. This is what any competent operator deploys.
- **C — GAP:** fresh context, reads `I + S_{k-1}`, emits envelope `d_k ≪ S_k`.

Decomposition: **B vs A** isolates the input/statelessness win (available to anyone). **C vs B** isolates the *defensible* output-envelope win. `r` scales how much the output win matters.

### 3.2 The baseline's strongest honest form (one canonical config, used for every headline)

Define **`baseline_strongest` = Scenario B + symmetric optimal cache + same model + equalized structured-output**:

1. **Symmetric cache (no thumb on the scale).** Under the hot-cache regime, B pins `I + S_{k-1}` into a cacheable segment and gets the **same** discount GAP gets on its maintain prefix. Cache-write is charged **symmetrically**: both flows re-write the changed artifact-in-message each turn, so **neither** gets a free artifact cache. GAP gets credit only for its stable maintain *system* prefix; B for its stable system+instruction prefix. Result: when both are optimally cached, GAP's net edge **collapses to the output-only term** — exactly the term we headline.
2. **Equalized structured-output tax.** GAP runs strict `json_schema` (`runner.rs:203`); B runs `json_schema=None`. Apples-to-oranges. Fix: measure the tax directly — run one GAP turn **with and without** strict schema, report the delta as `schema_decoding_overhead`, and **subtract it** before the C-vs-B output claim.
3. **Model parity (binding rule).** Every A/B/C cost comparison **holds the model fixed**. Effect 3 (cheaper maintain model, `gap.md:510`) appears only in a separate, clearly-labeled "Effect 3 sensitivity" section that grants the baseline the **identical** cheap-model Scenario-B substitution. The headline never combines fewer-tokens × cheaper-model.
4. **Fallback-aware by default.** On `apply_succeeded==false`, GAP pays the wasted envelope round-trip **plus ≥1 maintain retry plus the full base regen plus an orchestrator re-read penalty**. This is the **default headline**, not the optimistic number. **Hard rule:** an experiment with `apply_success_rate < 0.8` is reported as a **GAP loss**, never averaged into a positive headline.

### 3.3 Tool-use / agent loop (Effect 2, `gap.md:490-492`) — MODELED, not a new live flow

Effect 2 is GAP's largest win and the explicit goal, but a *live* loop with stubbed chat turns makes "savings" a function of how many fake turns the author inserts. **Build it as a MODELED projection from already-measured tokens**, parameterized by a *swept* `loop_shape`:

- Inputs are real: `S_k` (measured base output per turn) and the **real serialized handle size** from `apply()`'s returned `Envelope` (`apply.rs` returns `(Artifact, Envelope)`; serialize the second — its `targets` array can be 100–300 tokens for HTML, not a 10-token guess). **Serialize inside the eval**, never modify `src/apply.rs`.
- **Steelman baseline (DEFAULT) = `BaselineKeepLatest`**: artifact stored out-of-band, one current body in context, re-read **only on steps that touch it**. `BaselineAccumulate` (full body every step) is a **labeled worst-case** row only.
- `rereads_avoided` counted **only against steps that genuinely need artifact content**, never "all remaining steps."
- Report as a **curve over post-edit-turn-count `{0,2,5,10}`**, not a single %. The quadratic-vs-linear divergence *is* the result.
- **Two ledgers, never merged:** orchestrator wallet (transcript) vs maintain wallet (ephemeral edit context). Report each plus a combined total. A live `run_agentloop_flow` is deferred behind `--agentloop-live`.

### 3.4 Unified cost accounting — one source of truth, no double-counting

All cost math moves to **`apps/eval/src/cost.rs`** (out of `report.rs`). Per-turn, init-inclusive:

```
cost_k = (input_k − cached_k)·p_in + cached_k·p_cached_in + cache_write_k·p_write + output_k·p_out
total  = Σ_{k=0..N} cost_k          // INCLUDES turn-0; one function for run_single AND report.rs
```

Cache-write is charged **once per new cacheable segment** (hot) or **every turn** (cold), **symmetrically across flows**. Init is in every cumulative total for all three flows, exactly once. `cost.rs` math is pure → **unit-tested** (`break_even`, cumulative curves, fallback arithmetic).

---

## 4. Metrics to add

Every report cell is tagged **MEASURED** (tokens/latency/parse/apply) or **MODELED** ($/break-even/agent-loop) at **cell granularity**. `cache_observed` renders an em-dash when `cache_reported==false` (distinct from a true 0% hit).

| Metric | Unit | Where captured | Measured / Modeled |
|---|---|---|---|
| `scenario_b_flow` (stateless full regen) | tokens | `runner::run_stateless_flow` | MEASURED |
| `input_savings_b_vs_a_pct` | % | `cost.rs` | MEASURED (tokens) |
| `output_savings_c_vs_b_pct` | % | `cost.rs` | MEASURED (tokens) |
| `schema_decoding_overhead_tokens` | tokens | one with/without-schema GAP call | MEASURED |
| `shared_turn0_hash` / `strategy` | sha / enum | `experiment::run_single` | MEASURED (diagnostic) |
| `protocol_overhead_tokens` | tokens | `(gap_init_system.chars − base_system.chars)/4` | MODELED (char/4, labeled) |
| `apply_latency_us` | µs | `Instant` around `apply::apply` (Ok **and** Err) | MEASURED |
| `envelope_ops_count` | count | `envelope.content.len()` | MEASURED |
| `envelope_output_tokens` (`d_k`) | tokens | alias of gap `output_tokens` (stop misusing `output_bytes`) | MEASURED |
| `ttft_ms` / `decode_ms` / `decode_tps` | ms / tok·s⁻¹ | `client.rs`, `t0` re-anchored before `.send()` | MEASURED |
| `itl_p50/p90/p99_ms`, `stream_chunk_count`, `streaming_emulated` | ms / count / bool | `client.rs`; **NULL** when `chunk_count < output_tokens/4` | MEASURED (null when emulated) |
| `retried` (exclude from latency stats) | bool | `client.rs` retry loop | MEASURED |
| `cost_regimes{off, observed, theoretical_best, cold}` | USD | `cost.rs` | MODELED |
| `cache_reported` | bool | `client.rs` | MEASURED |
| `break_even_turn` (vs A and vs B, init-inclusive) | turn | `cost.rs` | MODELED |
| `break_even_r` (r below which GAP edge vanishes vs cached B) | scalar | `cost.rs` | MODELED |
| `d_over_s_ratio` (signed, unclamped, vs **B** same artifact) | ratio | `cost.rs` | MEASURED |
| `fallback_invoked`, `gap_effective_{input,output}_tokens` | bool / tokens | `runner` + `cost.rs` FALLBACK policy | MODELED |
| `run_degenerate` (output_bytes never changes) | bool | `run_single` gate | MEASURED |
| `base_input_monotone` (A must be non-decreasing) | bool | `run_single` gate | MEASURED |
| `structurally_valid` (per flow, per turn) | bool / None | `validators.rs` | MEASURED (None=UNMEASURED) |
| `intent_satisfied`, `effective_intent_satisfied` | bool | `assertions.rs` + `scorer.rs` | MEASURED (where authored) |
| `cost_per_correct_edit_usd` | USD | `cost.rs` ÷ correctness | MIXED (print both) |
| `full_artifact_rereads_avoided` (curve over loop_shape) | count | `cost.rs` agent-loop projection | MODELED |
| `win_rate` (fraction of reps GAP cost < baseline) | % | aggregation | MEASURED |
| `determinism_index` | scalar | `scorer.rs` | MEASURED |
| `Stat{mean,stddev,n,ci95_lo,ci95_hi,cv}`, `RateStat{k,n,wilson_lo,wilson_hi}` | — | aggregation (Student-t, Wilson) | MEASURED |

---

## 5. Report layout

```
# GAP Experiment Results
Model `{m}` | Experiments {n} | Reps {R} | Pricing in/cached/write/out | r={r}
Legend: MEASURED=tokens/latency/parse/apply · MODELED=$/break-even/agent-loop
Reproducibility: determinism_index base/gap · seed_supported={bool} temperature_active={bool}

## 1. Headline — Savings Decomposition (A/B/C)            [MEASURED tok, MODELED $]
"Going stateless (B vs A) cuts INPUT {x}%. Envelopes (C vs B, schema-tax-subtracted) cut
OUTPUT {y}%. Even granting the baseline a symmetric optimal cache + same model, GAP costs
{z}% less because output is never cached. Headline is MACRO (unweighted mean ±CI).
Whale-removed (top-3 dropped): {z'}%."

## 2. Per-experiment summary (MACRO default; mean±sd when R>1)
| Exp | Fmt | A in | B in | C in | A out | B out | C out (d) | mean d/S vs B | B-Intent | G-Intent | ΔQual | B-Valid | G-Valid | Cov | Parse(Wilson) | Apply(Wilson) | Fail% |

## 3. Cost — symmetric multi-regime, init-INCLUSIVE       [MODELED]
Per-flow 4-regime table (off/observed/theoretical_best/cold) + fallback-on-failure column.
GAP-savings matrix: regime × {vs A, vs B}. Spec-style cumulative (Init|+1|+5|+N).
r-sweep {1,2,3,5} × discount{none,0.5,0.1} computed **vs cached B** (the steelman).

## 4. Agent Loop — Effect 2 (orchestrator separation)     [MODELED projection]
Curve over post-edit turns {0,2,5,10}; baseline=KeepLatest (steelman), Accumulate=worst-case row.
Two-ledger block (orchestrator | maintain | combined). Real-handle-token note.

## 5. Latency                                             [MEASURED]
Per flow: TTFT/TTLT/wall-clock median+IQR (labeled "incl. network+queue, NOT pure prefill"),
decode_tps (should be SIMILAR across flows — diff = schema tax). ITL p50/p90 or "emulated".
Apply-engine row: apply p50 µs = F% of wall-clock. Retried-turn exclusion footnote.

## 6. When GAP is NOT worth it                            [MEASURED + MODELED]
Worst-first. d/S bins, break-even histogram, three-state worth-it flag, f*/N* thresholds.

## 7. Correctness vs Cost                                 [MEASURED quality, MODELED $]
Per flow: correct edits (effective n/N), validity rate, regressions, cost-per-correct.

## 8. Aggregation (Simpson-safe): MACRO default | micro (whale-labeled) | stratified by size/format/edit-type.
## 9. Flow divergence (informational): demoted seq_sim/F1/ROUGE-L with caveat.
## 10. Adversarial suite (segregated; never in headline).
## 11. Effect 3 sensitivity (cheap maintain model) — baseline granted SAME cheap-model B.
```

---

## 6. "When GAP is NOT worth it" — the explicit loss region

Reported worst-cases-first, as **three-state** flags (`worth_it / inconclusive / not_worth_it`), with CIs:

- **Tiny artifacts (`S` small):** envelope + JSON-schema tax + protocol overhead exceed output savings. `break_even_turn = null`. (cf. `026` above: input −74.8%.)
- **Full rewrites (`d ≈ S`, d/S ≈ 1):** full-regen is structurally cheaper; output savings ≈ 0.
- **Short lifecycles (`N=1`):** init overhead never amortizes; B wins. (Current suite is *all* 2–4 edits — see §8.)
- **Read-only / 0 edits:** GAP is pure overhead; `rereads_avoided=0`. Report honest zero.
- **High failure rate (`apply_success_rate < 0.8`):** fallback doubles cost; reported as a **loss**, not averaged in. (`026` is the canonical real example.)
- **Frontier models:** hypothesis "GAP output savings shrink and failures fall on stronger models" — full-regen of a 2k-token artifact is cheap and ~100% valid; GAP's structured-edit path is where failures concentrate.
- **Closed-form thresholds:** `break_even_r`, `f*` (failure rate above which fallback erases savings), `N*` (lifecycle length below which overhead never amortizes) — one each per experiment.

---

## 7. Adversarial / negative + tool-use experiments to add

Segregated from the headline average, counts reported separately:

- **tiny-json-flag-toggle** — ~150-token config, single value edit. Expect `not_worth_it`.
- **md-full-rewrite** — forces `d ≈ S`; output savings ≈ 0.
- **single-edit-discard** — `N=1`; B wins; overhead never amortized.
- **readonly-no-edit** — 0 edits; verifies honest zero, `rereads_avoided=0`.
- **css-section-replace** — envelope carries near-full content; exercises rewrite/inflation d/S bins.
- **html-long-agent-loop** — many post-edit turns on a large HTML artifact; **positive** Effect-2 case maximizing orchestrator divergence; pairs with readonly as the loss boundary.
- **malformed-target-negative** — targets that don't exist → drives apply failures; tests FALLBACK cost doubling, `f*`.
- **json-pointer-edit-suite** — structured edits checkable by `json_pointer` assertions; exercises intent over substring matching.

**Authoring policy for the existing suite:** ship pure-Rust validators (free structural validity, zero authoring) for **all** format-typed outputs first; author intent assertions only for the adversarial set + a stratified representative sample; let `assertion_coverage` expose the gap (intent rates below a coverage floor render UNMEASURED, with a selection-bias caveat).

---

## 8. Statistical rigor & fairness fixes

1. **Run-validity gates (P0, before any aggregation):**
   - `run_degenerate=true` if a flow's `output_bytes` never changes across edit turns → **excluded from headline** (`026` is exactly this: 8661×3, apply=0).
   - `base_input_monotone` must hold for Scenario A. When violated (`026`: 4082→624→2230), the provider reports **post-cache** counts → the cache model **must use those counts**, not pretend cache=0, and the experiment is flagged.
2. **Reps are P0, not P2.** No `break_even`/`not_worth_it`/decomposition value ships at `n=1`; gate behind `n≥3`; tag single-run fields `PROVISIONAL_N1`. Default `R=3` (CI), `R=5` aspirational; agent-loop/adversarial at `R=1` smoke.
3. **Paired-difference CIs.** All three flows run within the same rep; use **paired-difference** intervals (tighter), not two-independent-sample CIs.
4. **Correct intervals.** `Stat` via **Student-t** (n<30); rates via **Wilson**. Savings computed **per-rep then aggregated** (no ratio-of-means bias).
5. **MACRO is the default headline** (unweighted mean of per-experiment savings ± CI). micro (token-weighted) is demoted/labeled "dominated by largest artifacts." **Whale-removal check:** re-report with top-3 largest-output experiments dropped; if it moves >5pp, headline is whale-driven → stratified-only.
6. **`win_rate`** (fraction of reps where GAP cost < baseline per experiment) reported alongside mean savings.
7. **Latency honesty.** `t0` re-anchored immediately before the successful `.send()`. TTFT/decode reported median+IQR labeled "incl. network+queue, NOT pure prefill" (a black-box streaming API cannot decompose prefill vs decode). ITL **NULLed** when `streaming_emulated`. Retried turns excluded. `decode_tps` reported for **both** flows (should be similar — divergence is a schema tax).
8. **`determinism_index`** computed on the **canonicalized** artifact (serde round-trip for structured formats) or the **intent-pass vector**, not raw `seq_sim`. Note `seed=42` is likely ignored and gpt-5 forces `temperature→1.0`, so the headline model is near-maximally nondeterministic — reps are not optional.
9. **`protocol_overhead_tokens`** from a fixed char/4 prompt heuristic (deterministic), **never** from cache-contaminated observed turn-0 deltas.
10. **Per-experiment flags are descriptive, not inferential** (reserve CIs for the aggregate). `cost_per_correct_edit` prints numerator/denominator separately with a Wilson CI on the denominator.

---

## 9. Prioritized roadmap P0→P2 (file/function change map)

Ship in **two independent change sets** to avoid a giant signature-churning PR. **Keep the existing flat `Metrics` struct extended with `Option` fields — do NOT introduce `schema_version:2` until SET 2.** Cheap, no-new-API-call wins first; the high-churn shared-turn-0 last.

### SET 1 — the headline (≈4 files, no protocol-crate change, no on-disk schema break)

| Step | File · function | Change | Priority |
|---|---|---|---|
| 1 | **`cost.rs` (NEW)** | Move `Price`/`price_for`/`flow_cost` out of `report.rs`. Add init-inclusive `cumulative_cost_curve`, `break_even`, cache regimes (`off`, `theoretical_best` with **symmetric** credit/write), `break_even_r`. **Unit tests.** Zero new API calls + fixes the init-exclusion bug. | **P0** |
| 2 | **`runner.rs` · `run_stateless_flow`** (NEW) | Mirror GAP's edit-loop input construction **exactly** but `json_schema=None`, full-regen output, seeded from shared turn-0. Unlocks B-vs-A / C-vs-B decomposition. | **P0** |
| 3 | **`runner.rs`** edit loop | Wrap `apply::apply` in `Instant` → `apply_latency_us` (Ok **and** Err). Set `envelope_ops_count = envelope.content.len()`, `envelope_output_tokens = r.output_tokens`. Serialize the returned `_handle` Envelope for real handle-token size. | **P0** |
| 4 | **`runner.rs`** edit loop | One with/without-schema GAP call → `schema_decoding_overhead_tokens`. | **P0** |
| 5 | **`cost.rs`** | MODELED agent-loop projection from measured `S_k` + real handle tokens; `BaselineKeepLatest` (default) vs `BaselineAccumulate` (worst-case); curve over `loop_shape ∈ {0,2,5,10}`; two ledgers. | **P1** |
| 6 | **`client.rs`** | Re-anchor `t0` before successful `.send()`. Replace `median_itl_ms` with `itl_p50/p90/p99` (NULL when emulated), `stream_chunk_count`, `decode_ms`, `decode_tps`, `retried`, `cache_reported`. | **P1** |
| 7 | **`experiment.rs`** | `run_stateless_flow` branch; make all three flows the **same `FlowMetrics` type** (move `envelope_parse_rate`/`apply_success_rate` to `Option<f64>` on `FlowMetrics`, drop `GapFlowMetrics` wrapper); `Decomposition`; **run-validity gates** (`run_degenerate`, `base_input_monotone`); extend `TurnResult`/`TurnMetrics`. | **P0/P1** |
| 8 | **`main.rs`** | `--flow` as a parsed set `{base,stateless,gap,agentloop}` with aliases `both={base,gap}`, `abc={base,stateless,gap}`, `all=+agentloop`. Add `--force` (defeat the `experiment.rs` resume skip so new flows run). | **P0** |
| 9 | **`report.rs`** | Consume `cost.rs`; render sections 1/3/4/5; back-compat for legacy flat `metrics.json`. Swap Seq-Sim/F1 columns for decomposition + intent. | **P0/P1** |

### SET 2 — correctness & statistical rigor

| Step | File · function | Change | Priority |
|---|---|---|---|
| 10 | **`validators.rs` (NEW)** | `validate(format,text)` via `serde_json`/`serde_yaml`/`toml`/`quick-xml`; HTML via lenient parser. External `py_compile`/`node --check`/`gofmt` when on PATH else `None` (UNMEASURED, never pass-by-default). Free structural validity for all experiments, zero authoring. | **P2 (high value)** |
| 11 | **`assertions.rs` (NEW)** | `TurnAssertions` from optional `assertions.json`; `json_pointer` numeric-normalized; absence reduces `assertion_coverage`, never auto-passes. | **P2** |
| 12 | **`scorer.rs`** | `score_flow` scoring **each flow vs ground truth** (not vs each other); gate intent on `edit_applied`; demote `seq_sim/f1/rouge_l` to "divergence (informational)"; `determinism_index` on canonicalized/intent vectors; make `strip_gap_markers` pub. | **P2** |
| 13 | **`experiment.rs`** | `--reps` loop; `Stat`/`RateStat` (Student-t, Wilson); per-rep-then-aggregate; `win_rate`; `{repetitions,runs,aggregate}` + `schema_version:2`; paired-difference CIs. | **P2** |
| 14 | **`runner.rs` · `generate_turn0`** (NEW) | Extract; call once per rep; base/stateless seed from **stripped** artifact, GAP from **marked**; assert `shared_turn0_hash` equality. **Split strategy default** (plain for A/B, marked for C); marker overhead charged to GAP. Highest churn → last. | **P2** |
| 15 | New adversarial experiments; FALLBACK-on-failure default; Effect-3 sensitivity section; model tier (one mid, one frontier). | | **P2** |
| 16 | **`EXPERIMENT.md`** | Reconcile with this doc: A/B/C decomposition, symmetric-cache rule, schema-tax subtraction, model-parity, FALLBACK default, two-ledger Effect 2, run-validity gates, TTFT anchor + ITL nulling, MACRO headline. | **P2** |

`Cargo.toml`: add `sha2` (turn-0 hash), `toml`/`serde_yaml`/`quick-xml` (validators). **No tokenizer crate** (provider-mismatched for free-tier endpoints; `protocol_overhead_tokens` stays char/4-labeled). **No change to `src/apply.rs`** — serialize the handle inside the eval.

---

## 10. Open questions

1. **Shared turn-0 single-vs-split:** Split (plain A/B, marked C) keeps marker overhead on GAP per the spec but reintroduces a small turn-0 divergence; single-marked gives the baseline GAP's tidy structure (may bias its edits/quality). **Recommendation:** split as canonical default + a 5-experiment ablation under both. (Shared turn-0 only fixes turn-0; turns 1..N still diverge as flows drift, so d/S **vs B same-turn artifact** is the trustworthy ratio.)
2. **Frontier-model tier vs free-tier budget:** `R=5 × ~90 × ~4 turns × 3 flows` is thousands of calls under rate limits; the thesis may *invert* on frontier models. **Recommendation:** `R=3` default, structured-format calibration at `R=5`, at minimum one mid + one frontier model on a stratified sample.
3. **Cache calibration without a paid key:** free tiers return no `cached_tokens`, so `theoretical_best` is unfalsifiable. **Recommendation:** run a 5-experiment calibration on a provider that reports `cached_tokens`, assert the model within ~5%; else the report prints "theoretical cache regime UNCALIBRATED on this provider."
4. **Agent-loop canonical shape:** report the curve over `{0,2,5,10}` post-edit turns as the result; no single point.
5. **Effect 3 (two-model) scope:** defer to the labeled "Effect 3 sensitivity" section; default holds model fixed.
6. **Assertion authoring scale:** validators (free, all experiments) now; intent assertions for adversarial + stratified sample; `assertion_coverage` + selection-bias caveat expose the rest.
7. **Resolve the `026` input anomaly before any input-axis claim:** re-run `026`-class experiments after the schema/apply work and confirm `apply_success_rate > 0` and `base_input_monotone` before citing any input-axis or decomposition number.
