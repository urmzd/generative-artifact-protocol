build:
    cargo build

test:
    cargo test

# Rust criterion micro-benchmarks (apply engine performance)
bench-rust:
    cargo bench

# Generate benchmark table and embed into README
bench-protocol:
    cargo run --release --bin bench_table > benches/results.md

# Generate experiment input directories (no LLM needed)
bench-generate count="0":
    cd evals && uv run aap-evals generate $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi)

# Run a single experiment (requires Ollama)
bench-single n="1" model="qwen3.5:4b":
    cd evals && uv run aap-evals run --single {{n}} --model {{model}}

# Run all experiments
bench model="qwen3.5:4b" count="0":
    cd evals && uv run aap-evals run --model {{model}} $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi)

# Generate apply-engine benchmark corpus (artifacts + envelopes)
bench-generate-apply count="0" model="gemma4":
    python3 scripts/generate_apply_benchmarks.py --model {{model}} $(if [ "{{count}}" != "0" ]; then echo "--count {{count}}"; fi)

# Evaluation reports
eval-cost:
    cd evals && uv run aap-evals eval-cost

eval-reliability:
    cd evals && uv run aap-evals eval-reliability

eval-similarity:
    cd evals && uv run aap-evals eval-similarity

eval: eval-cost eval-reliability eval-similarity

bench-all: bench-rust bench-protocol
