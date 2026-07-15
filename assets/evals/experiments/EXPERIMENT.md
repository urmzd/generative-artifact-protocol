# GAP Conversation Benchmark

This suite measures the Generative Artifact Protocol against full-regeneration
baselines on real, multi-turn LLM edit sessions. Each experiment is a directory;
the `evalset` Go package materializes those directories as `github.com/urmzd/saige/eval`
observations. Existing `metrics.json` files are committed historical measurements
and can be scored offline through SAIGE scorers.

## Hypotheses

1. **Output tokens drop 90–99%** on edit turns — envelopes carry only the change, not the artifact.
2. **Input is bounded** — the stateless maintain context reads `instructions + current artifact`, not the growing conversation.
3. **Output savings survive prompt caching** — caching mostly erases the baseline's *input* disadvantage, but output tokens are never cached and dominate cost, so GAP's edge persists.
4. **Apply is effectively free** — envelope resolution is ~2 µs, dominated by LLM time.
5. **Reliability and correctness are measurable** - and misses are part of the cost model, not something to hide.

## Design

### Three flows, identical edits

Every experiment runs the same turn-0 creation and the same edit instructions through up to three flows (`--flow base|stateless|gap|both|abc|all`):

| Flow | Scenario | Context per edit turn | Output |
|---|---|---|---|
| **base** | **A** | full conversation history (all prior artifacts + messages accumulate) | full artifact, regenerated |
| **stateless** | **B** | `system + current artifact + edit` (fresh each turn) | full artifact, regenerated |
| **gap** | **C** | `GAP maintain prompt + current artifact + edit` (fresh each turn) | a JSON **edit envelope**, applied by the engine |

This separates the two effects that a naive "GAP vs base" comparison conflates:

- **B vs A — the statelessness/input win.** Any baseline can drop the growing history; this is not unique to GAP.
- **C vs B — GAP's defensible output win.** Holding statelessness constant, the only difference is full regeneration vs a small envelope. This is the protocol's contribution.

The report renders this as the **A/B/C decomposition** (only when the stateless flow is run, i.e. `--flow abc`/`all`).

> **Turn 0.** The `base` and `stateless` flows operate on the same plain creation artifact. The `gap` flow's turn-0 carries `<gap:target>` markers (or is structured for JSON-Pointer targeting), so it is a slightly larger, instrumented variant — that marker overhead is reported honestly, not hidden.

### The system prompt is the independent variable

GAP defines a wire format; the **producer's system prompts** decide whether a model uses it well — and whether edits are even *correct*. Each experiment carries two GAP prompts:

- `inputs/gap/init-system.md` — **synthesis** prompt. Must elicit fine-grained, role-based markers (text/markup) or clean pointer-addressable JSON. A weak prompt ("wrap updatable values") collapses the artifact into one coarse marker; a later edit then targets that coarse region and replaces — **destroying** — everything else, while `apply` reports success. Strengthening this prompt alone took a complex HTML case from 1 marker (correctness ≈ 0) to 58 markers (correct, surgical edits) at the *same* ~99% output savings.
- `inputs/gap/maintain-system.md` — **edit** prompt. References existing target IDs / pointers, puts replacement text in each op's `content` field, never re-emits the full artifact.

**Targeting is format-aware** (the apply engine supports both):

| Artifact family | Mode | Synthesis output |
|---|---|---|
| HTML, code, Markdown, YAML, XML, SVG, CSS, … | `<gap:target>` markers | instrumented text |
| `application/json` | JSON Pointer (RFC 6901) | clean JSON, no markers |

### Controlled variables

| Variable | Value |
|---|---|
| Model | same for all flows (eliminates capability differences) |
| Temperature | `0` for chat models; **omitted** for reasoning models (o-series, gpt-5 family) which reject it |
| Seed | `42` where the provider supports it |
| Edit instructions | byte-identical across flows |
| Transient failures | retried with exponential backoff (429 / 5xx) so one blip doesn't abort a run |

The GAP system prompt is the intervention and is **not** held constant — its token cost is the protocol overhead and is reported, not excluded.

## Directory layout

Each experiment directory `<NNN>-<name>/` contains:

- `README.md`: the `**Format:** <mime>` line is parsed by the SAIGE evalset loader
- `inputs/base/system.md`: "You produce \<mime\> artifacts. Output raw code only."
- `inputs/base/turn-0.md`: creation prompt
- `inputs/base/turn-1.md` … `turn-N.md`: one edit instruction per file
- `inputs/gap/init-system.md`: synthesis prompt (markers OR pointer mode)
- `inputs/gap/maintain-system.md`: edit prompt
- `checks/turn-1.json` … `turn-N.json`: correctness oracles (optional, see below)
- `outputs/base/turn-k.<ext>`: regenerated artifact per turn (Scenario A)
- `outputs/stateless/turn-k.<ext>`: regenerated artifact per turn (Scenario B, `--flow abc`/`all`)
- `outputs/gap/turn-k.json` + `turn-k.<ext>`: envelope + resolved artifact per turn (Scenario C)
- `metrics.json`: committed measurements from prior runs
- `../../saige/observations.json`: generated SAIGE observation set for all experiments

## Correctness oracles (high-fidelity scoring)

Reliability metrics like "apply succeeded" only check that the engine ran — they do **not** notice a successfully-applied edit that emptied the document. `checks/turn-N.json` closes that gap with deterministic assertions evaluated against the produced artifact (GAP **and** base, on identical oracles):

```json
{
  "turn": 3,
  "checks": [
    {"kind": "valid_json"},
    {"kind": "contains", "value": "$215,430"},
    {"kind": "absent",   "value": "$182,000"},
    {"kind": "regex_count", "pattern": "\"id\"\\s*:", "expected": 100},
    {"kind": "regex_count_at_least", "pattern": "<tr", "min": 40},
    {"kind": "json_pointer_equals", "pointer": "/pagination/page", "value": 3}
  ]
}
```

The `regex_count` "exact item count" assertion is the key signal — it catches a run that applied cleanly but dropped the other items. SAIGE subjects that execute these observations should write a `correctness {pass_rate, base_pass_rate, per_turn}` block into their metrics output.

## Miss Economics and Honest Expectations

GAP is a protocol optimization with a real failure mode. A benchmark that only reports "output token savings" is incomplete because the production question is not "was the envelope short?" It is "did the envelope parse, apply, preserve correctness, and still save tokens after fallback?"

The live runner treats a missed GAP edit as a failed attempt. It records the failed envelope call, leaves the current artifact unchanged for that turn, and marks the turn with `failed`, `failure_reason`, `envelope_parsed`, and `apply_succeeded`. New metrics also include:

- `reliability.miss_count` and `reliability.miss_rate`
- miss taxonomy: parse miss, invalid envelope, apply miss, request failure, unknown miss
- `economics.measured_total_token_savings_pct`: raw GAP attempt savings on edit turns
- `economics.fallback_adjusted`: assumes every missed GAP edit is retried as a full baseline regeneration
- `economics.amortized`: includes turn-0 marker/protocol overhead, not only edit turns

The expected production behavior should be conservative:

- **Clean case:** the envelope parses, applies, and passes correctness checks. Savings are real.
- **Parse or invalid-envelope miss:** the model failed to produce usable protocol output. The safe response is a full-regeneration retry, so the miss costs the failed GAP call plus the baseline retry.
- **Apply miss:** the model targeted something missing or structurally incompatible. The safe response is also fallback. These misses usually mean the synthesis prompt did not create stable, granular targets or the maintain prompt selected the wrong target.
- **False success:** `apply_succeeded = true`, but the artifact lost unrelated content or violated the edit intent. This is the most dangerous case because protocol metrics look healthy. Correctness oracles and artifact-level checks are required.
- **Degenerate run:** the artifact never changes. Savings are illusory and should be excluded from headline aggregates.

Fallback-adjusted savings are the honest number to use when talking about expected cost in a product. Raw savings are still useful for measuring the protocol ceiling, but they should be reported next to miss rate and correctness. A high-savings, high-miss run is a prompt or targeting problem, not a production-ready result.

## Dependent variables (measured)

| Metric | Per turn | Notes |
|---|---|---|
| `input_tokens`, `output_tokens` | ✓ | from provider usage |
| `cached_input_tokens` | ✓ | prompt-cache hits; powers the cache-on vs cache-off cost model |
| `latency_ms`, `ttft_ms`, `ttlt_ms`, `median_itl_ms` | ✓ | wall-clock + streaming timings |
| `envelope_parsed`, `apply_succeeded` | ✓ (GAP) | wire-level reliability |
| `reliability.miss_rate` | aggregate | failed GAP edit attempts divided by edit turns |
| `economics.fallback_adjusted.*_savings_pct` | aggregate | savings after modeling full-regeneration fallback on misses |
| `economics.amortized.*_savings_pct` | aggregate | init-inclusive session savings, including marker/protocol overhead |
| `mean_sequence_similarity`, `token_f1`, `rouge_l` | aggregate | similarity of GAP result to the baseline |
| correctness `pass_rate` / `base_pass_rate` | ✓ | from `checks/` oracles |

Reports derived from these observations should preserve the same dependent variables: output/input savings, **A/B/C decomposition**, **caching-aware, init-inclusive cost**, miss economics, latency summaries, run-validity gates, and correctness tables.

## Multi-item / multi-page experiments (`101`–`108`)

Beyond the per-format basics, experiments `101`–`108` stress **large, paginated artifacts** — 80–200 items across multiple pages — across HTML, JSON (pointer mode), Markdown, YAML, XML/RSS, Python, SQL, and TypeScript. Their five edit turns exercise the hard cases: change one field of an item on a deep page, insert at a position, delete from the middle, **bulk-change a field across all items**, and add a whole new page. Each turn has a `checks/turn-N.json` that pins the exact post-edit item count, so collateral loss is caught immediately.

## Interpreting results

**Success:** output savings > 90% on edit turns; flat GAP input vs growing base input; parse rate > 80%; low miss rate; fallback-adjusted savings still positive; **correctness on par with the base flow** (GAP didn't trade fidelity for tokens); positive C-vs-B output decomposition.

**Failure / investigate:** correctness far below base correctness means edits are dropping or corrupting content, usually from too-coarse targets in `init-system.md`; parse rate < 50% means the model cannot reliably produce the envelope format; high apply-miss rate means target IDs or JSON pointers are unstable; positive raw savings with weak fallback-adjusted savings means misses are eating the economic benefit; degenerate GAP runs make "savings" illusory and should be excluded.

## Running

```sh
# Regenerate the SAIGE observation set
just evalset

# Validate the loader and run SAIGE scorers over committed metrics
go test ./evalset

# Full repository gate
just check
```

Live LLM runs should be implemented as SAIGE subjects over
`assets/evals/saige/observations.json`, then scored with SAIGE scorers and GAP's
committed-metrics scorer helpers.
