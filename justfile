build:
    cargo build

test:
    cargo test

# Rust criterion micro-benchmarks (apply engine speed)
bench:
    cargo bench

# Build the eval CLI (release for speed)
build-eval:
    cargo build --release -p gap-eval

# Run benchmark experiments. flow ∈ base|stateless|gap|both|abc|all
run count="0" model="" id="" flow="both" api-base="" api-key="": build-eval
    ./target/release/gap-eval run \
        --experiments-dir assets/evals/experiments \
        $(if [ "{{model}}" != "" ]; then echo "--model {{model}}"; fi) \
        $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi) \
        $(if [ "{{id}}" != "" ]; then echo "--id {{id}}"; fi) \
        --flow {{flow}} \
        $(if [ "{{api-base}}" != "" ]; then echo "--api-base {{api-base}}"; fi) \
        $(if [ "{{api-key}}" != "" ]; then echo "--api-key {{api-key}}"; fi)

# Generate report from experiment metrics
report: build-eval
    ./target/release/gap-eval report --experiments-dir assets/evals/experiments

# Retroactive quality scoring
score: build-eval
    ./target/release/gap-eval score --experiments-dir assets/evals/experiments

# Re-evaluate correctness oracles (checks/turn-N.json) on completed runs
checks: build-eval
    ./target/release/gap-eval checks --experiments-dir assets/evals/experiments
