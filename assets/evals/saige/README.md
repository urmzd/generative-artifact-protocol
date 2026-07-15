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

The previous standalone runner has been removed. New live runners should use
SAIGE subjects against these observations and score results with SAIGE scorers.
