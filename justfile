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

# Run conversation benchmark experiments (base vs GAP flows)
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
