# GAP Experiment Results

**Model:** `test-model` | **Experiments:** 3

| Experiment | Fmt | Base Out | GAP Out | Out Δ | Parse | Apply | Seq Sim | F1 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 001-html-healthy | text/html |   4050 |    200 |  95.1% | 2/2 | 2/2 | 0.965 | 0.940 |
| 002-json-degenerate † | applicatio |   1800 |    150 |  91.7% | 2/2 | 0/2 | — | — |
| 003-yaml-abc-oracles | text/x-yam |   1520 |    130 |  91.4% | 1/1 | 1/1 | 0.990 | 0.980 |

† degenerate run (artifact never changed — all edits no-ops); excluded from the savings and cost aggregates below.

**Output savings (2 eligible / 3 runs):**

| Aggregate | Savings |
|---|---:|
| Micro (token-weighted; dominated by large artifacts) | 94.1% |
| Macro (mean of per-experiment savings; outlier-sensitive) | 93.3% |
| Median per-experiment | 93.3% |

**Reliability (all 3 runs, degenerate included):** Parse: 5/5 | Apply: 3/5

## Run validity

- ⚠ **1/3** GAP runs are **degenerate** (artifact never changed — all edits no-ops). Their "output savings" are illusory; they are excluded from every savings and cost aggregate in this report.

## Savings decomposition (A/B/C, init-inclusive, MEASURED tokens)

Mean per-experiment savings over eligible (non-degenerate) runs. **B vs A** = the input win from going stateless (any baseline can adopt it). **C vs B** = GAP's defensible output-envelope win.

| Axis | Comparison | Mean savings |
|---|---|---:|
| Input | B vs A (statelessness) | 28.6% |
| Output | C vs B (edit envelopes) | 44.2% |
| Input | C vs A | 12.2% |
| Output | C vs A | 44.4% |

## Cost analysis — init-inclusive, cache regimes (MODELED $)

Over 2 eligible experiments (degenerate runs excluded). Prices (USD/1M): input $1.000, cached-input $1.000, output $4.000. Output is never cached.

| Flow | Cost (cache off) | Cost (cache observed) | Cost (cache theoretical-best) |
|---|---:|---:|---:|
| Base (Scenario A, full regen) | $0.0465 | $0.0465 | $0.0465 |
| GAP (Scenario C, envelopes) | $0.0247 | $0.0247 | $0.0247 |

**GAP savings vs base:** 46.9% (cache off) → 46.9% (base perfectly cached, GAP not).
Even with a perfectly hot cache on the baseline, GAP's advantage survives — the residual is the output-token win, which no cache can discount.

**Break-even** (cumulative GAP cost < perfectly-cached base): reached in 2/2 eligible experiments, mean edit turn 1.0.

## Agent loop — Effect 2 (orchestrator context separation), MODELED

Orchestrator-wallet input tokens summed across eligible experiments, as the orchestrator spends extra reasoning turns holding the artifact. **KeepLatest** (steelman baseline) keeps only the current body in context; **Accumulate** (worst case) retains every version; **GAP** holds a handle. This is a *separate ledger* from the edit work above (the maintain wallet = Scenario C).

| Extra turns | KeepLatest in | Accumulate in | GAP in | Re-reads avoided | GAP savings vs KeepLatest | GAP $ vs KeepLatest $ |
|---:|---:|---:|---:|---:|---:|---:|
| +0 | 9070 | 16570 | 560 | 5 | 93.8% | $0.0006 vs $0.0091 |
| +2 | 16210 | 34710 | 1020 | 9 | 93.7% | $0.0010 vs $0.0162 |
| +5 | 26920 | 61920 | 1710 | 15 | 93.6% | $0.0017 vs $0.0269 |
| +10 | 44770 | 107270 | 2860 | 25 | 93.6% | $0.0029 vs $0.0448 |

Across 2 eligible experiments. The KeepLatest column grows linearly with reasoning turns and Accumulate quadratically, while GAP stays flat — every re-read avoided is a full artifact body the orchestrator never pays to re-ingest.

## Latency (median over edit turns, IQR in parens, MEASURED)

Wall-clock includes network + queueing, not pure prefill/decode. Retried turns are excluded (their wall-clock contains rate-limit backoff). TTLT is also reported as a mean; long-tail turns pull it above the median.

| Flow | Turns | TTFT | TTLT | TTLT mean | Total latency |
|---|---:|---:|---:|---:|---:|
| Base (full regen) | 5 | 860 ms (740-1050) | 6000 ms (4800-9050) | 6740 ms | 6200 ms (5000-9250) |
| Stateless (full regen) | 1 | 855 ms (855-855) | 5900 ms (5900-5900) | 5900 ms | 6100 ms (6100-6100) |
| GAP (envelopes) | 5 | 980 ms (790-1225) | 1500 ms (1275-1800) | 1530 ms | 1700 ms (1450-2000) |

⚠ Turns recorded without a `retried` flag predate the harness fix and their TTFT/TTLT are **under revision** (methodology corrected 2026-06-09: the stream timer was anchored after response headers, so TTFT/TTLT excluded request upload and queue time, and retried turns cannot be identified or filtered). Values are kept visible until the suite is re-run under the corrected harness.

## Correctness oracles (checks/turn-N.json — multi-item/multi-page fidelity)

Pass rate = fraction of per-turn assertions satisfied: targeted change present, old/deleted values gone, and EXACT item count preserved (collateral-loss detector). GAP vs BASE evaluated on identical oracles.

| Experiment | Fmt | GAP correct | Base correct |
|---|---|---:|---:|
| 003-yaml-abc-oracles | text/x-yaml | 100% | 67% |

**Mean correctness:** GAP 100.0% | Base 66.7% (n=1)
