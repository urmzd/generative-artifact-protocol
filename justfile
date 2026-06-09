build:
    cargo build

test:
    cargo test

# Format check, lints, and tests (same gate as CI)
check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Rust criterion micro-benchmarks (apply engine speed)
bench:
    cargo bench

# Build the eval CLI (release for speed)
build-eval:
    cargo build --release -p gap-eval

# Run benchmark experiments. flow ∈ base|stateless|gap|both|abc|all
run count="0" model="" id="" flow="both" api-base="" api-key="": build-eval
    #!/usr/bin/env sh
    set -eu
    set -- run --experiments-dir assets/evals/experiments --flow '{{flow}}'
    [ '{{count}}' = "0" ] || set -- "$@" --count '{{count}}'
    [ -z '{{model}}' ] || set -- "$@" --model '{{model}}'
    [ -z '{{id}}' ] || set -- "$@" --id '{{id}}'
    [ -z '{{api-base}}' ] || set -- "$@" --api-base '{{api-base}}'
    [ -z '{{api-key}}' ] || set -- "$@" --api-key '{{api-key}}'
    ./target/release/gap-eval "$@"

# Regenerate the committed experiment report from metrics.json files
report: build-eval
    ./target/release/gap-eval report --experiments-dir assets/evals/experiments --output assets/evals/experiments/results.md
    @echo "wrote assets/evals/experiments/results.md"

# Payload-size table (wire + content bytes) from the apply-engine fixtures
payload-report: build-eval
    ./target/release/gap-eval payload-report

# Retroactive quality scoring
score: build-eval
    ./target/release/gap-eval score --experiments-dir assets/evals/experiments

# Re-evaluate correctness oracles (checks/turn-N.json) on completed runs
checks: build-eval
    ./target/release/gap-eval checks --experiments-dir assets/evals/experiments
