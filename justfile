build:
    go build ./...

test:
    go test ./...

# Materialize the GAP corpus as SAIGE eval observations
evalset:
    go run ./internal/evalsetgen

# Format check, vet, and tests (same gate as CI)
check:
    test -z "$(gofmt -l *.go evalset/*.go internal/evalsetgen/*.go)"
    go run ./internal/evalsetgen
    git diff --exit-code -- assets/evals/saige/observations.json
    go vet ./...
    go test ./...
