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
just evalset      # regenerate assets/evals/saige/observations.json
go test ./evalset # validate loader and score committed metrics through SAIGE
just check        # full repository gate, including evalset drift check
```

## Rules

- Do not hand-edit `assets/evals/saige/observations.json`; run `just evalset`.
- Do not hand-edit measured `metrics.json` or `results.md` values.
- Live LLM runs should be implemented as SAIGE subjects over the generated
  observations and scored with SAIGE scorers plus `evalset.MetricsScorers()`.
