<p align="center">
  <h1 align="center">Generative Artifact Protocol</h1>
  <p align="center">
    Token-efficient artifact updates for LLMs.
    <br /><br />
    <a href="https://pkg.go.dev/github.com/urmzd/generative-artifact-protocol">Go Docs</a>
    &middot;
    <a href="https://github.com/urmzd/generative-artifact-protocol/issues">Report Bug</a>
    &middot;
    <a href="spec/gap.md">Specification</a>
  </p>
</p>

<p align="center">
  <a href="https://github.com/urmzd/generative-artifact-protocol/actions/workflows/ci.yml"><img src="https://github.com/urmzd/generative-artifact-protocol/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  &nbsp;
  <a href="LICENSE"><img src="https://img.shields.io/github/license/urmzd/generative-artifact-protocol" alt="License"></a>
</p>

> **Warning**: This project is `v0`: the protocol, schemas, and APIs are subject to breaking changes without notice until a formal release.

Generative Artifact Protocol (GAP) lets LLMs update structured text artifacts by emitting small JSON envelopes instead of regenerating full files. The native implementation is now Go-only: protocol types, `<gap:target>` marker resolution, JSON Pointer edits, a stateless apply engine, and a versioned in-memory store live in the root Go package.

## Features

- **Envelope system**: `synthesize`, `edit`, and `handle` operations
- **Stateless apply engine**: `Apply(base, envelope) -> artifact, handle`
- **ID-based targeting**: `<gap:target id="ID">` markers for text formats
- **JSON Pointer targeting**: pointer edits for `application/json`
- **Format-agnostic model**: HTML, Markdown, source code, JSON, YAML, SVG, SQL, and other text artifacts
- **Optimistic versioning**: edit envelopes apply only against the expected previous version
- **SSE transport binding**: wire format for streaming with reconnection support in [GAP-SSE](spec/gap-sse.md)

## Installation

```sh
go get github.com/urmzd/generative-artifact-protocol
```

From source:

```sh
git clone https://github.com/urmzd/generative-artifact-protocol
cd generative-artifact-protocol
just test
```

## Quick Start

```go
package main

import (
	"encoding/json"
	"fmt"

	gap "github.com/urmzd/generative-artifact-protocol"
)

func main() {
	format := "text/html"
	body := `<gap:target id="msg">hello</gap:target>`
	raw, _ := json.Marshal(gap.SynthesizeContentItem{Body: body})

	envelope := gap.Envelope{
		Protocol: gap.ProtocolVersion,
		ID:       "greeting",
		Version:  1,
		Name:     gap.NameSynthesize,
		Meta:     gap.Meta{Format: &format},
		Content:  []json.RawMessage{raw},
	}

	artifact, handle, err := gap.Apply(nil, envelope)
	if err != nil {
		panic(err)
	}
	fmt.Println(artifact.Body)
	fmt.Println(handle.Name)
}
```

## Apply Engine

```go
artifact, handle, err := gap.Apply(baseArtifact, envelope)
```

| Envelope | Direction | Description |
|---|---|---|
| `synthesize` | input | Full artifact content, usually the initial version |
| `edit` | input | Targeted changes via marker ID or JSON Pointer |
| `handle` | output | Lightweight reference returned after every mutation |

The apply engine performs no network or filesystem I/O. It returns errors for malformed envelopes, missing targets, invalid JSON pointers, out-of-bounds array indexes, and unsupported input operation names.

## Store

Use `ArtifactStore` when you want version checks and bounded history:

```go
store := gap.NewArtifactStore(10)
artifact, handle, err := store.Apply(envelope)
```

For edit envelopes, the store enforces `stored_version == envelope.version - 1`. A `synthesize` envelope can create or reset an artifact chain.

## Commands

| Recipe | Description |
|---|---|
| `just build` | Compile the Go package |
| `just test` | Run Go tests |
| `just check` | Run formatting, vet, and tests |

Equivalent direct commands:

```sh
go test ./...
go vet ./...
test -z "$(gofmt -l *.go)"
```

## Evaluation Corpus

The historical evaluation datasets remain under [`assets/evals/`](assets/evals/) for reproducibility and future Go runner work. The previous eval binary and benchmark harness have been removed in favor of a Go-only core. Committed `metrics.json` and `results.md` files should still be treated as measured artifacts: do not hand-edit measured numbers.

## License

This project is dual-licensed:

- **Code** (Go files, build files): [Apache License 2.0](LICENSE)
- **Specification, eval assets, and documentation**: [CC-BY 4.0](LICENSE-CC-BY-4.0)

See [NOTICE](NOTICE) for details. Attribution is required under both licenses.
