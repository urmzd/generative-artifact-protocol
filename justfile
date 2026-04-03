build:
    cargo build

test:
    cargo test

# Rust criterion micro-benchmarks (apply engine speed)
bench:
    cargo bench

# Sync workspace — builds Rust FFI via maturin + all Python packages
bind:
    uv sync

# Generate benchmark corpus (artifacts + envelopes via LLM)
generate count="0" model="" id="" provider="google" fallback="":
    uv run aap-evals generate --provider {{provider}} $(if [ "{{model}}" != "" ]; then echo "--model {{model}}"; fi) $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi) $(if [ "{{fallback}}" != "" ]; then echo "--fallback {{fallback}}"; fi)

# Run conversation benchmark experiments (base vs AAP flows)
run count="0" model="" id="" provider="google" fallback="":
    uv run aap-evals run --provider {{provider}} $(if [ "{{model}}" != "" ]; then echo "--model {{model}}"; fi) $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi) $(if [ "{{id}}" != "" ]; then echo "--id {{id}}"; fi) $(if [ "{{fallback}}" != "" ]; then echo "--fallback {{fallback}}"; fi)

# Generate markdown report from experiment metrics
report:
    uv run aap-evals report
