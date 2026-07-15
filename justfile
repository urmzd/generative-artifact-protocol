build:
    go build ./...

test:
    go test ./...

# Materialize the GAP corpus as SAIGE eval observations
evalset:
    go run ./internal/evalsetgen

# Regenerate the eval results report from committed/live metrics
report:
    go run ./internal/evalreport

# Run live OpenAI-compatible eval experiments. flow ∈ base|stateless|gap|both|abc|all
run count="0" model="" id="" flow="both" api-base="" api-key="" force="false":
    #!/usr/bin/env sh
    set -eu
    set -- --experiments-dir assets/evals/experiments --flow '{{flow}}'
    [ '{{count}}' = "0" ] || set -- "$@" --count '{{count}}'
    [ -z '{{model}}' ] || set -- "$@" --model '{{model}}'
    [ -z '{{id}}' ] || set -- "$@" --id '{{id}}'
    [ -z '{{api-base}}' ] || set -- "$@" --api-base '{{api-base}}'
    [ -z '{{api-key}}' ] || set -- "$@" --api-key '{{api-key}}'
    [ '{{force}}' = "false" ] || set -- "$@" --force
    go run ./cmd/gap-eval "$@"

# Format check, vet, and tests (same gate as CI)
check:
    test -z "$(gofmt -l *.go cmd/gap-eval/*.go evalset/*.go internal/evalsetgen/*.go internal/evalreport/*.go internal/liveeval/*.go)"
    go run ./internal/evalsetgen
    git diff --exit-code -- assets/evals/saige/observations.json
    go run ./internal/evalreport
    git diff --exit-code -- assets/evals/experiments/results.md
    go vet ./...
    go test ./...
