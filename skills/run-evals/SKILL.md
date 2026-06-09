---
name: run-evals
description: Run the GAP LLM evaluation suite and regenerate its reports. Use when running benchmark experiments against a model, rerunning a single experiment, scoring quality or correctness retroactively, or regenerating results.md and the payload table from committed metrics. Live runs need an API key; reporting and scoring work offline from committed data.
---

# Run GAP Evals

The eval harness (`gap-eval`, built from `apps/eval/`) measures GAP envelope edits against full-regeneration baselines on real LLM runs. All recipes live in the `justfile`; `just run` builds the release binary first.

## Prerequisites

- Rust (stable) and [just](https://github.com/casey/just).
- For live runs only: an OpenAI-compatible endpoint and API key. Reporting (`just report`, `just payload-report`) and scoring (`just score`, `just checks`) need no key.

## Environment variables

| Variable | Meaning |
|---|---|
| `GAP_API_BASE` | OpenAI-compatible base URL (default `https://api.openai.com/v1`) |
| `GAP_API_KEY` | API key for that endpoint |

If `GAP_API_KEY` is unset, the CLI falls back through `OPENAI_API_KEY`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GROQ_API_KEY`, `CEREBRAS_API_KEY`, `OPENROUTER_API_KEY`, `MISTRAL_API_KEY`, `GITHUB_TOKEN`, in that order. The repo `.envrc` loads both from a gitignored `.env` via direnv.

## Providers (OpenAI-compatible endpoints)

| Provider | `GAP_API_BASE` |
|---|---|
| OpenAI | `https://api.openai.com/v1` |
| Google AI Studio (Gemini) | `https://generativelanguage.googleapis.com/v1beta/openai/` |
| Groq | `https://api.groq.com/openai/v1` |
| Cerebras | `https://api.cerebras.ai/v1` |
| OpenRouter | `https://openrouter.ai/api/v1` |
| Mistral La Plateforme | `https://api.mistral.ai/v1` |
| GitHub Models | `https://models.inference.ai.azure.com` |

Free tiers carry per-minute and per-day caps; batch large runs with the `count` and `id` arguments. See the README "Running evals on a free tier" section for tier details.

## Running experiments

`just run [count] [model] [id] [flow] [api-base] [api-key]` (positional, all optional):

```sh
just run 5 "gemini-2.5-flash"                # first 5 experiments, base+gap flows
just run 0 "gpt-4o-mini" 026                 # one experiment by ID prefix, all turns
just run 5 "gemini-2.5-flash" "" abc         # abc = base+stateless+gap, enables A/B/C decomposition
just run 1 "deepseek/deepseek-chat:free" "" both \
  "https://openrouter.ai/api/v1" "$OPENROUTER_API_KEY"
```

- `count` 0 means all experiments; `flow` is one of `base`, `stateless`, `gap`, `both` (default), `abc`, `all`.
- Experiments with an existing `metrics.json` are skipped; pass `--force` via the binary directly (`./target/release/gap-eval run --force ...`) to re-run.
- Each run writes `outputs/<flow>/turn-N.*` and `metrics.json` into the experiment directory, then scores quality and correctness automatically.

## Regenerating reports

```sh
just report          # rewrites assets/evals/experiments/results.md from all committed metrics.json
just payload-report  # wire/content byte table from assets/evals/apply-engine fixtures
just score           # retroactive quality (LCS/F1/ROUGE) + correctness scoring
just checks          # re-evaluate checks/turn-N.json oracles only
```

Never hand-edit numbers in `results.md` or the README's measured tables; regenerate them.

## Reading the results

- `metrics.json` (one per experiment): `default_flow` (Scenario A), `stateless_flow` (B), `gap_flow` (C) each hold `per_turn` token/latency rows; `gap_flow` adds `envelope_parse_rate` and `apply_success_rate`. `validity.gap_run_degenerate = true` flags a run whose artifact never changed (excluded from headline aggregates). `quality` holds similarity scores, `correctness` the oracle pass rates, `decomposition` the A/B/C savings split. Turns with `retried = true` carry rate-limit backoff in their wall-clock and are excluded from latency aggregates.
- `results.md`: the rendered summary. Micro savings are token-weighted (dominated by large artifacts); macro and median rows give the per-experiment view; the whale check drops the 3 largest artifacts. Cost figures are MODELED from measured tokens; token and latency figures are MEASURED. Degenerate runs render footnoted with a dagger.
