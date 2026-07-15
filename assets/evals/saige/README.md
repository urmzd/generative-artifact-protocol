# SAIGE Eval Set

This directory contains the GAP benchmark corpus materialized as
`github.com/urmzd/saige/eval` observations.

- `observations.json`: one SAIGE observation per multi-turn GAP experiment
- observation `input`: prompts, format metadata, expected sections, and path refs
- observation `annotations`: stable `gap.*` keys for metrics, outputs, checks, and source directory

Regenerate the set after changing `assets/evals/experiments/`:

```sh
just evalset
```

Run live OpenAI-compatible experiments with:

```sh
just run 0 "gpt-4o-mini" "004" both
```

The previous standalone Rust runner has been removed. Live runs now use the Go
`cmd/gap-eval` runner and write outputs plus `metrics.json` back into each
experiment directory. SAIGE can consume the generated observations and score
committed metrics with `evalset.MetricsScorers()`.

When reporting results, keep raw savings separate from production expectations.
New live `metrics.json` files include `reliability` and `economics` blocks for
miss count/rate, fallback-adjusted savings, retry tax, and init-inclusive
amortized savings. Treat raw `comparison` savings as the protocol attempt and
fallback-adjusted savings plus correctness as the honest product view.
