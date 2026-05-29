<p align="center">
  <h1 align="center">Generative Artifact Protocol</h1>
  <p align="center">
    Token-efficient artifact updates and streaming for LLMs — up to 99% fewer output tokens per edit.
    <br /><br />
    <a href="https://crates.io/crates/generative-artifact-protocol">Crates.io</a>
    &middot;
    <a href="https://github.com/urmzd/generative-artifact-protocol/issues">Report Bug</a>
    &middot;
    <a href="spec/gap.md">Specification</a>
  </p>
</p>

<p align="center">
  <a href="https://github.com/urmzd/generative-artifact-protocol/actions/workflows/ci.yml"><img src="https://github.com/urmzd/generative-artifact-protocol/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/generative-artifact-protocol"><img src="https://img.shields.io/crates/v/generative-artifact-protocol" alt="crates.io"></a>
  &nbsp;
  <a href="LICENSE"><img src="https://img.shields.io/github/license/urmzd/generative-artifact-protocol" alt="License"></a>
</p>

> **Warning**: This project is `v0` — the protocol, schemas, and APIs are subject to breaking changes without notice until a formal release.

An open standard protocol — **[GAP](spec/gap.md)** — that lets LLMs declare, diff, and reprovision text artifacts with minimal token expenditure. Includes a Rust reference implementation of the apply engine plus an evaluation CLI for measuring token efficiency against real LLM runs.

## Features

- **Envelope system** — three operation types (`synthesize`, `edit`, `handle`) for full generation, targeted updates, and lightweight references
- **Stateless apply engine** — pure function, no I/O, ~2μs per edit; portable to browsers (WASM), IDEs, CLIs, or service backends
- **ID-based targeting** — `<gap:target id="ID">` markers and JSON Pointer paths eliminate hallucinated search strings
- **Format-agnostic** — works with HTML, Python, JavaScript, JSON, YAML, Rust, Go, SVG, and more
- **Up to 99% output token reduction** per edit — actual reduction depends on edit scope: ~95-99% for value changes, ~80-95% for section rewrites; total cost savings depend on model pricing ratio and edit history ([cost model](spec/gap.md#711-cost-model))
- **SSE transport binding** — wire format for streaming with reconnection support ([GAP-SSE](spec/gap-sse.md))
- **Evaluation framework** — 90+ experiment datasets (including multi-item / multi-page artifacts) measuring token efficiency, prompt-cache-aware cost, reliability, and **per-turn correctness** against real LLM runs ([how it works](#benchmark--evaluation-suite))

## Install

**Rust crate:**

```sh
cargo add generative-artifact-protocol
```

**From source (full workspace):**

```sh
git clone https://github.com/urmzd/generative-artifact-protocol
cd generative-artifact-protocol
```

Requires [Rust](https://rustup.rs/) (stable) and optionally [just](https://github.com/casey/just) (for recipes).

## Quick Start

```sh
# Build the Rust library
just build

# Run tests
just test

# Run criterion benchmarks (apply engine speed)
just bench

# Build the eval CLI (release)
just build-eval

# Run LLM evaluations
just run count=5 model="gemini-2.0-flash"

# Generate report from experiment metrics
just report
```

## Usage

### How it works

```
LLM ──produces──▶ envelope ──apply──▶ (artifact, handle)
                                 ▲
                           gap (stateless, ~2μs)
```

1. An LLM produces an artifact envelope (JSON) — either a `synthesize` envelope (full content with target markers) or an `edit` envelope (targeted changes by ID or JSON Pointer).
2. The apply engine resolves the envelope against the current artifact state to produce the updated artifact and a lightweight handle.
3. The orchestrator holds handles; the resolved artifact is stored and consumed by downstream tools — browsers, IDEs, etc.

### Apply engine

The core of the library is a single stateless function:

```rust
pub fn apply(artifact: Option<&Artifact>, envelope: &Envelope) -> Result<(Artifact, Envelope)>
```

| Envelope | Direction | Description |
|---|---|---|
| **synthesize** | input | Complete artifact content (baseline or reset) with `<gap:target>` markers |
| **edit** | input | Targeted changes via ID (`<gap:target>` markers) or JSON Pointer |
| **handle** | output | Lightweight reference returned after every synthesize or edit |

### Recipes

| Recipe | Description |
|---|---|
| `just build` | Compile the Rust library |
| `just test` | Run Rust unit tests |
| `just bench` | Criterion micro-benchmarks (apply engine speed) |
| `just build-eval` | Build the eval CLI (release) |
| `just run [count] [model] [id] [flow] [api-base] [api-key]` | Run benchmark experiments. `flow` ∈ `base`, `stateless`, `gap`, `both` (default), `abc` (all three — enables A/B/C decomposition), `all` |
| `just report` | Generate markdown report from experiment metrics (token savings, cache-aware cost, decomposition, correctness) |
| `just score` | Retroactive quality + correctness scoring of experiment results |
| `just checks` | Re-evaluate correctness oracles (`checks/turn-N.json`) on completed runs |

### Benchmark & evaluation suite

The repo ships a full multi-turn benchmark that measures GAP against full-regeneration baselines on real LLM runs. Datasets live in [`assets/evals/experiments/`](assets/evals/experiments/EXPERIMENT.md), one directory per experiment:

```
<NNN>-<name>/
├── README.md                 # **Format:** <mime> | edits | design notes
├── inputs/base/              # system.md + turn-0 (creation) + turn-1..N (edits)
├── inputs/gap/               # init-system.md (synthesis) + maintain-system.md (edits)
├── checks/turn-N.json        # correctness oracles for each edit turn (optional)
├── outputs/{base,stateless,gap}/  # produced artifacts (+ GAP envelopes) per turn — committed for browsing
└── metrics.json              # all measurements (regenerated by each run)
```

Every flow writes its turn-by-turn output to `outputs/<flow>/` — `turn-N.<ext>` for the resolved artifact and, for GAP, `turn-N.json` for the raw edit envelope. A sample set of experiments ships these outputs committed, so you can open one on GitHub and read exactly what each flow produced at every turn (and diff `base` vs `gap` directly); running the suite populates `outputs/` and `metrics.json` for the rest. `just report` then aggregates every `metrics.json` into one Markdown summary.

**Three flows, identical edits** (`--flow base|stateless|gap|both|abc|all`):

| Flow | Scenario | What it does |
|---|---|---|
| `base` | A | Stateful conversation — full regeneration each turn, history accumulates |
| `stateless` | B | *Steelman baseline* — stateless full regeneration (reads only the current artifact) |
| `gap` | C | Stateless GAP envelope edits applied by the engine |

This lets the report **decompose** GAP's win: **B vs A** is the input/statelessness win (any baseline can adopt it); **C vs B** is GAP's defensible output-envelope win.

**What's measured.** Output/input token savings, reported as an **A/B/C decomposition** so the input win (going stateless, B vs A) is separated from GAP's defensible output-envelope win (C vs B); **prompt-cache-aware, init-inclusive cost** priced under three regimes — caching `off`, `observed` (provider-reported hits), and a steelman `theoretical-best` that grants the baseline a perfectly hot cache and GAP none (even then GAP wins, because output tokens are never cached) — plus the **break-even turn** at which GAP's cumulative cost overtakes the baseline; **latency** (TTFT, TTLT, total wall-clock per flow); envelope parse/apply reliability; content similarity to the baseline; and **per-turn correctness oracles** — `checks/turn-N.json` assertions (`contains`, `absent`, `regex_count`, `json_pointer_equals`) that verify the targeted change landed, deleted values are gone, and the **exact item count** is preserved. The last is the high-fidelity signal: it catches a run that "applied successfully" but silently dropped items.

**Tool-use / orchestrator separation (Effect 2).** The report also projects the *orchestrator's* input tokens across an agent loop. When a tool generates an artifact, a full-regeneration system returns the whole body into the conversation — re-read on every subsequent turn, and re-read again whenever the data changes. GAP returns a lightweight **handle**, so the orchestrator's context stays flat no matter how large the artifact or how many times it is edited and re-referenced. The projection compares `KeepLatest` (steelman baseline — only the current body in context, grows linearly), `Accumulate` (worst case — every version retained, grows quadratically), and GAP (handle, flat) over a curve of reasoning turns, counting the full-body re-reads GAP avoids.

**Run-validity gates.** Each run is checked for degeneracy — if the GAP artifact never changes across edit turns (every edit a no-op, usually because applies failed) it is flagged and **excluded from headline aggregates**, so a broken run can't report illusory savings. A non-monotone base input is also flagged (it means the provider reports post-cache token counts).

**Multi-item / multi-page artifacts.** Experiments `101`–`108` generate 80–200 items across multiple pages (HTML catalog, paginated JSON, Markdown reference, YAML manifests, RSS, Python/SQL/TS records) and edit a specific item on a deep page, insert/delete across pages, and bulk-change a field across all items — the cases where targeting precision matters most.

> **⚠ Results depend on the synthesis system prompt.** GAP only works if the producer is prompted to emit **fine-grained, role-based target markers** at creation time (or clean, pointer-addressable JSON). A weak prompt collapses the artifact into one coarse marker, so edits replace — and destroy — the whole document while reporting success. Strengthening the prompt alone took a complex HTML case from ~0 to high correctness with the same model and the same ~99% token savings. Targeting is format-aware: markers for text/code, JSON Pointer for `application/json`. See [spec §5.1](spec/gap.md#producer-system-prompt-requirements).

### Measured results

A run across **18 experiments** spanning 17 formats on `gpt-5.4-mini` (full report: [`results.md`](assets/evals/experiments/results.md)). These are real measurements, not the projections in the [cost model](#cost-model) below — and they include the cases where GAP loses.

| Finding | Result |
|---|---|
| **Output-token reduction** | **81% in aggregate** across the suite (token-weighted; per-experiment 64–95% on well-suited artifacts, with one outlier loss below) |
| **Cost vs full-regen baseline** | **54.5% cheaper** with caching off → **45.9% cheaper even when the baseline is *perfectly* cached** and GAP is not. The surviving gap is the output-token win, which no cache can discount. |
| **Break-even** | GAP's cumulative cost overtakes the baseline by **edit ~1.3** on average (11/18 experiments) |
| **End-to-end latency** | **~4× faster** — mean time-to-last-token 3.4 s vs 16.6 s (far fewer output tokens to stream) |
| **Tool-use / orchestrator (Effect 2)** | over a 10-turn agent loop, **~80% less orchestrator input** than keeping the latest artifact body in context — 240 full-body re-reads avoided |
| **Envelope reliability** | parse **56/56 (100%)**; apply **47/56 (84%)** — applies are the weak point, concentrated in a few formats |

**Where it doesn't pay off (kept in the headline, not hidden).** On a tiny SQL schema (`057`), GAP emitted **10× *more* tokens** than a full rewrite (−859% "savings"): when the artifact is small and the edit rewrites most of it, an envelope is pure overhead and full regeneration wins. One run was flagged **degenerate** (every apply failed → the artifact never changed) and excluded from the headline. The full loss region — tiny artifacts, full rewrites, single-edit lifecycles, high failure rates — is characterized in [`apps/eval/STEELMAN.md`](apps/eval/STEELMAN.md).

### Running evals on a free tier

The eval CLI speaks the OpenAI chat completions wire format, so any OpenAI-compatible endpoint works. Point it at a free-tier provider with `GAP_API_BASE` and `GAP_API_KEY` (or `--api-base` / `--api-key`). The CLI also picks up `OPENAI_API_KEY` as a fallback.

| Provider | Base URL | Free-tier highlights |
|---|---|---|
| **Google AI Studio** (Gemini) | `https://generativelanguage.googleapis.com/v1beta/openai/` | Persistent free tier on `gemini-2.5-flash` / `gemini-2.0-flash` — recommended starting point |
| **Groq** | `https://api.groq.com/openai/v1` | Free `llama-3.3-70b-versatile`, `qwen3-32b`, etc.; sub-second TTFT, ~30 RPM |
| **Cerebras** | `https://api.cerebras.ai/v1` | Free `llama-4-scout`, `qwen-3-32b`; fastest tokens/sec on the list |
| **OpenRouter** | `https://openrouter.ai/api/v1` | 25+ models suffixed `:free` (e.g. `deepseek/deepseek-chat:free`); aggregates many providers |
| **Mistral La Plateforme** | `https://api.mistral.ai/v1` | Free "Experiment" tier — 1 B tokens/month at ~2 RPM |
| **GitHub Models** | `https://models.inference.ai.azure.com` | Free preview for developers; OpenAI, Llama, Mistral, Phi |
| **Cloudflare Workers AI** | `https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/ai/v1` | Free daily quota across Llama, Mistral, Qwen |

Example — run five experiments against Gemini for free:

```sh
export GAP_API_BASE="https://generativelanguage.googleapis.com/v1beta/openai/"
export GAP_API_KEY="$GEMINI_API_KEY"
just run count=5 model="gemini-2.5-flash"
```

Example — run a single experiment against an OpenRouter free model:

```sh
just run count=1 model="deepseek/deepseek-chat:free" \
  api-base="https://openrouter.ai/api/v1" api-key="$OPENROUTER_API_KEY"
```

> **Tip — model asymmetry for cheaper runs.** GAP's maintain context only needs recall and structured output. A frontier model for `init` paired with a free or near-free model for `maintain` matches the [model asymmetry](spec/gap.md#71-memory-model) the cost model assumes. Today the CLI runs both flows with one model — for asymmetric runs, invoke `gap-eval run --flow gap` against the cheaper provider after a baseline pass with the stronger one.

> **Caveat.** Free tiers carry per-minute and per-day caps. For the full 90-experiment suite, expect to throttle or batch with `--count` and `--id`.

### Cost model

GAP saves tokens by replacing full artifact regeneration with small edit envelopes. The actual savings are **not deterministic** — they depend on edit scope, how efficiently the LLM generates envelopes, the model's tokenizer, and pricing. The LLM may produce larger-than-minimal edits or fall back to full regeneration. The estimates below assume well-formed, targeted edits. See the [full derivation in the spec](spec/gap.md#711-cost-model).

In a naive conversation, each turn's input carries everything: instructions, all prior artifact versions, all prior messages, and the current request — growing quadratically with edit count. GAP's maintain context is **stateless**: each call starts fresh with only the instructions ($I$), the current artifact ($S$), and the edit request. It produces a small edit envelope ($d$ output tokens) and terminates. The apply engine resolves the edit deterministically (CPU, ~2μs, zero tokens). Three effects compound:

- **Input token reduction:** a naive conversation at edit $k$ reads $I + S_0 + S_1 + \ldots + S_{k-1}$ — every prior version. GAP reads only $I + S_{k-1}$ — the current artifact. At 10 edits, this is ~78% fewer input tokens.
- **Output token reduction:** the LLM produces $d$ tokens instead of $S$ per edit — only the changed content plus a constant envelope overhead (JSON structure, target IDs). $d$ scales with the size of the change, not the size of the artifact. A two-value edit on a 2,000-token artifact produces ~30 tokens; a section rewrite produces hundreds. Either way, unchanged content is never regenerated.
- **Model asymmetry:** the maintain context can use a cheaper model than the orchestrator, multiplying savings further.

**Projected example** — using a reference model at \$3/M input, \$15/M output ($r = 5$), with a 2,000-token artifact, 30-token edit envelope, and 500 instruction tokens:

| After $N$ edits | Naive conversation | GAP | Estimated savings |
|---:|---:|---:|---:|
| 1 | \$0.069 | \$0.039 | ~43% |
| 5 | \$0.279 | \$0.071 | ~75% |
| 10 | \$0.677 | \$0.111 | ~84% |

> These figures assume ideal edit behavior ($d = 30$ tokens per edit). Actual savings vary — larger edits, section-level rewrites, or model-specific tokenization will shift these numbers. Savings also scale with $r$: at $r = 1$ (equal pricing), per-edit savings drop to ~44%; at $r = 5$, they reach ~79%.

### Payload benchmarks

Payload size and apply time for each envelope type, measured against an 8 KB HTML dashboard fixture.

> **Note:** "Payload savings" measures **byte reduction** — a proxy for output token reduction but not identical (tokenizers vary). See [cost model](spec/gap.md#711-cost-model) for the full derivation.

| Envelope | Scenario | Payload | % of Full | Payload savings | Apply Time |
|---|---|---:|---:|---:|---:|
| **synthesize** | Full generation (baseline) | 8,164 B | 100.0% | — | 1 ns |
| **edit** | 1 value replace (ID targeting) | 12 B | 0.1% | **99.9%** | 1.5 µs |
| **edit** | 4 value replaces (ID targeting) | 50 B | 0.6% | **99.4%** | 3.5 µs |
| **edit** | 1 section replace (ID targeting) | 441 B | 5.4% | **94.6%** | 1.4 µs |
| **edit** | 2 section replaces (ID targeting) | 516 B | 6.3% | **93.7%** | 3.8 µs |

## Limitations

1. **Conflict resolution** — GAP uses optimistic concurrency: the apply engine rejects edits whose version doesn't match `stored_version + 1`. Concurrent edits are rejected, not merged. There is no CRDT, OT, or automatic merge strategy — coordination is left to the orchestrator.

2. **Envelope generation** — The LLM must produce well-formed envelopes with correct target IDs, valid JSON structure, and appropriate operation types. In practice, generation can fail: malformed envelopes, hallucinated target IDs, or the model falling back to full regeneration when a targeted edit would suffice. The spec mitigates recall errors by passing a closed `targets` list in handles ([Section 7.2](spec/gap.md#72-handle-name-handle)), but reliable envelope production remains an active area of work.

3. **Granularity & the system prompt** — GAP's effectiveness is gated by the producer's synthesis prompt, not just the wire format. Too coarse — e.g. a single document-level marker — and a small edit replaces (and **destroys**) the whole region while `apply` still reports success; too fine and the target set grows, raising targeting-error and prompt-complexity costs. The mitigation is an explicit synthesis prompt that mandates per-value, role-based markers (or clean pointer-addressable JSON) — see [spec §5.1](spec/gap.md#producer-system-prompt-requirements). The eval suite measures this directly via per-turn [correctness oracles](#benchmark--evaluation-suite); robust fallback when a valid envelope can't be produced remains an active area of work.

4. **Adoption** — GAP requires tooling on both sides: producers must emit valid envelopes, consumers must implement the apply engine. The protocol is open and the spec is public, but the ecosystem is early.

## License

This project is dual-licensed:

- **Code** (`src/`, `apps/`, `benches/`, build files) — [Apache License 2.0](LICENSE)
- **Specification & docs** (`spec/`, `assets/`, documentation) — [CC-BY 4.0](LICENSE-CC-BY-4.0)

See [NOTICE](NOTICE) for details. Attribution is required under both licenses.
