# Contributing

Contributions are welcome. Please follow these guidelines.

## Getting started

```sh
git clone https://github.com/urmzd/generative-artifact-protocol
cd generative-artifact-protocol
just build   # compile the Rust library
just test    # run unit tests
just check   # format check, clippy lints, and tests (same gate as CI)
just bench   # run criterion benchmarks
```

## Project structure

- `src/lib.rs`: crate root (re-exports modules)
- `src/gap.rs`: envelope data model
- `src/apply.rs`: stateless apply engine
- `src/store.rs`: versioned artifact store
- `src/markers.rs`: section marker utilities
- `src/cffi.rs`: C FFI bindings
- `apps/eval/`: Rust eval CLI (`gap-eval`): experiment runner, scorer, reporter
- `assets/evals/`: evaluation datasets and experiments
- `spec/gap.md`: the protocol specification (wire version `gap/0.1`)
- `spec/gap-sse.md`: SSE wire format binding
- `spec/schemas/`: JSON Schema files
- `spec/examples/`: example envelopes
- `benches/`: criterion benchmarks
- `justfile`: task recipes
- `.github/workflows/`: CI (build + test) and release

## Making changes

- **Apply engine** (`src/apply.rs`): stateless function that resolves envelopes. Keep it pure: no I/O, no side effects.
- **Store** (`src/store.rs`): versioned artifact store with control-plane envelopes.
- **Eval CLI** (`apps/eval/`): Rust CLI for running LLM experiments, scoring, and reporting.
- **New recipes**: add them to `justfile` with a comment describing what they do.

## Pull requests

1. Fork the repo and create a branch from `main`.
2. Make your changes and ensure `just check` passes.
3. Open a pull request with a clear description of what changed and why.

## Code style

- Rust: `cargo fmt` before committing.

## License

By contributing you agree that your contributions will be licensed under the [Apache 2.0 License](LICENSE).
