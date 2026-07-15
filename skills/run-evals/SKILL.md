---
name: run-evals
description: Run or regenerate the GAP eval set after the Go/SAIGE migration. Use when validating the eval corpus, regenerating SAIGE observations, or scoring committed GAP metrics offline.
---

# Run GAP Evals

GAP evals are now represented as `github.com/urmzd/saige/eval` observations.
The source corpus is `assets/evals/experiments/`; the generated set is
`assets/evals/saige/observations.json`.

## Commands

```sh
just evalset                       # regenerate assets/evals/saige/observations.json
just run 0 "gpt-4o-mini" "004" both # live OpenAI-compatible run by ID prefix
go test ./evalset                  # validate loader and score committed metrics through SAIGE
just check                         # full repository gate, including evalset drift check
```

## Rules

- Do not hand-edit `assets/evals/saige/observations.json`; run `just evalset`.
- Do not hand-edit measured `metrics.json` or `results.md` values.
- Live LLM runs use the Go `cmd/gap-eval` runner and write `metrics.json` plus
  `outputs/` into the selected experiment directory.
- SAIGE consumers can read the generated observations and score metrics with
  `evalset.MetricsScorers()`.
- Report raw `comparison` savings together with `reliability` and `economics`.
  The honest product view is fallback-adjusted savings, miss rate, and
  correctness. A missed GAP edit costs the failed envelope attempt and usually a
  full-regeneration fallback retry.
