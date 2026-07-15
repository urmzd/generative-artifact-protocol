# Contributing

Contributions are welcome. Please follow these guidelines.

## Getting started

```sh
git clone https://github.com/urmzd/generative-artifact-protocol
cd generative-artifact-protocol
just build   # compile the Go package
just test    # run unit tests
just evalset # regenerate the SAIGE eval observation set
just check   # format check, vet, and tests (same gate as CI)
```

## Project structure

- `go.mod`: module definition
- `types.go`: envelope and protocol data model
- `apply.go`: stateless apply engine
- `store.go`: versioned artifact store
- `markers.go`: section marker utilities
- `evalset/`: SAIGE eval observation loader and committed-metrics scorers
- `assets/evals/`: evaluation datasets and experiments
- `assets/evals/saige/`: generated SAIGE observation set
- `assets/evals/experiments/go.mod`: module boundary that keeps generated fixtures out of root tests
- `spec/gap.md`: the protocol specification (wire version `gap/0.1`)
- `spec/gap-sse.md`: SSE wire format binding
- `spec/schemas/`: JSON Schema files
- `spec/examples/`: example envelopes
- `justfile`: task recipes
- `.github/workflows/`: CI (build + test) and release

## Making changes

- **Apply engine** (`apply.go`): stateless function that resolves envelopes. Keep it pure: no I/O, no side effects.
- **Store** (`store.go`): versioned artifact store with control-plane envelopes.
- **Eval assets** (`assets/evals/`): committed datasets and measured reports. Do not hand-edit measured numbers or generated SAIGE observations; run `just evalset`.
- **New recipes**: add them to `justfile` with a comment describing what they do.

## Pull requests

1. Fork the repo and create a branch from `main`.
2. Make your changes and ensure `just check` passes.
3. Open a pull request with a clear description of what changed and why.

## Code style

- Go: run `gofmt -w *.go *_test.go evalset/*.go internal/evalsetgen/*.go` before committing.

## License

By contributing you agree that your contributions will be licensed under the [Apache 2.0 License](LICENSE).
