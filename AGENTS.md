# AGENTS.md

Guidance for AI coding agents working in this repository. Humans should start with [README.md](README.md).

## Identity

Generative Artifact Protocol (GAP): an open protocol that lets LLMs declare, diff, and reprovision text artifacts with minimal token spend. The repo contains the normative spec, a Rust reference implementation of the apply engine, and an evaluation harness that measures the protocol against real LLM runs.

## Architecture

One cargo workspace, two crates:

- `generative-artifact-protocol` (lib name `gap`, published to crates.io). The envelope wire model lives in `src/gap.rs`, `<gap:target>` marker resolution in `src/markers.rs`, the stateless apply engine in `src/apply.rs` (a pure function: no I/O, must return errors and never panic), the versioned artifact store in `src/store.rs`, and C FFI bindings in `src/cffi.rs`.
- `gap-eval` (`apps/eval/`, internal, `publish = false`). Experiment loading and orchestration in `experiment.rs`, the three flow runners in `runner.rs`, the OpenAI-compatible streaming client in `client.rs`, LCS/F1/ROUGE scorers in `scorer.rs`, correctness oracles in `checks.rs`, the cost model in `cost.rs`, report rendering in `report.rs`, and the fixture payload table in `payload.rs`.

The normative spec is `spec/gap.md` (wire version `gap/0.1`); `spec/gap-sse.md` binds it to SSE. JSON Schemas sit in `spec/schemas/` and example envelopes in `spec/examples/`. Eval datasets live under `assets/evals/`: one directory per experiment with committed `metrics.json` files, plus the apply-engine fixtures used by `benches/gap.rs` and `just payload-report`.

`gap-eval` is an internal harness, not a portfolio CLI. It is exempt from the cli-standards rules (no self-update command, no global `--format` flag requirement).

Use `rg` to discover the current layout and call sites; do not rely on any static file listing being complete.

## Commands

| Command | What it does |
|---|---|
| `just check` | Format check, clippy `-D warnings`, and tests (the CI gate) |
| `just build` / `just test` | Compile the library / run tests |
| `just bench` | Criterion micro-benchmarks of the apply engine |
| `just run [count] [model] [id] [flow] [api-base] [api-key]` | Run LLM eval experiments (needs an API key; see `skills/run-evals/SKILL.md`) |
| `just report` | Regenerate `assets/evals/experiments/results.md` from committed `metrics.json` |
| `just payload-report` | Wire/content byte table from the apply-engine fixtures |
| `just score` / `just checks` | Retroactive quality scoring / correctness oracle re-evaluation |

Full verification gate before finishing any change:

```sh
cargo fmt && cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo bench --no-run
```

## Code style

- Rust 2021, `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` enforced in CI.
- Errors via `anyhow` with `context`/`with_context`; library code in `src/` bails with errors instead of panicking (a model can produce any input).
- Comments only for non-obvious constraints. Doc comments on public items.
- Fixed regexes go in `std::sync::LazyLock` statics, not per-call `Regex::new`.

## Measured numbers discipline

Never edit a measured number by hand in README.md or `results.md`. Regenerate them: `just report` rewrites `results.md` from the committed `metrics.json` files, and `just payload-report` plus `cargo bench` reproduce the payload table. Keep MEASURED and MODELED figures labeled as such. Degenerate runs (validity gate `gap_run_degenerate`) stay visible but are excluded from headline aggregates; `report.rs` enforces this and a golden test (`apps/eval/tests/fixtures/report/`) locks the rendering.

## Extension guide

Adding an experiment (full conventions in `assets/evals/experiments/EXPERIMENT.md`):

1. Create `assets/evals/experiments/<NNN>-<name>/` with a `README.md` containing a `**Format:** <mime>` line.
2. Add `inputs/base/system.md`, `inputs/base/turn-0.md`, and `inputs/base/turn-1.md`..`turn-N.md`. Optional `inputs/gap/init-system.md` and `maintain-system.md` override the spec-derived defaults.
3. Optionally add `checks/turn-N.json` correctness oracles (`valid_json`, `contains`, `absent`, `regex_count`, `regex_count_at_least`, `json_pointer_equals`).
4. Run it: `just run 0 <model> <NNN>`.

Adding a check oracle kind: extend the `Check` enum and `evaluate_check` in `apps/eval/src/checks.rs`, add the `kind()` arm, and unit-test the new variant in the same file.

Adding a supported format: extend `format_to_ext` in `apps/eval/src/experiment.rs`. Text formats get `<gap:target>` markers automatically; only `application/json` uses pointer addressing.
