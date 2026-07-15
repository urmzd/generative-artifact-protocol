# generative-artifact-protocol

Guidance for AI coding agents working in this repository. Humans should start with [README.md](README.md).

## Identity

Generative Artifact Protocol (GAP): an open protocol that lets LLMs declare, diff, and reprovision text artifacts with minimal token spend. The repo contains the normative spec, a native Go core apply engine, and historical eval datasets.

## Architecture

This is a root Go module:

| File | Role |
|---|---|
| `types.go` | Envelope, artifact, target, edit operation, and handle wire types |
| `markers.go` | `<gap:target>` marker construction, range lookup, nesting support, and target extraction |
| `apply.go` | Stateless apply engine for synthesize/edit envelopes |
| `store.go` | Versioned in-memory artifact store with checksum and rollback |
| `assets/evals/` | Historical experiment corpus and committed metrics |
| `spec/` | Normative GAP and GAP-SSE specifications plus JSON Schemas |

`assets/evals/experiments/go.mod` fences generated Go artifacts from the root module. Those files are eval outputs, not packages to compile.

## Commands

```sh
just build      # go build ./...
just test       # go test ./...
just check      # gofmt check + go vet + go test
go test ./...   # direct test command
```

## Code Style

- Go 1.25.
- Standard library only for the core engine unless a dependency is clearly justified.
- Small public API: exported protocol types, `Apply`, marker helpers, and `ArtifactStore`.
- Return errors instead of panicking. Model-produced input can be malformed, inconsistent, or adversarial.
- Use table-driven tests for new behavior and preserve parity with the protocol spec.
- Comments should explain non-obvious protocol constraints, not restate code.

## Measured Numbers Discipline

The eval assets contain committed measurements from the removed eval harness. Keep MEASURED and MODELED figures labeled as such. Do not hand-edit `metrics.json` or `results.md`; future measurement updates should come from a Go eval runner.
