"""Compare semantic similarity between base and AAP artifacts.

For turn 0 (creation), the base and AAP flows produce artifacts independently.
This script measures how similar they are — structural diff, embeddings cosine
similarity, or LLM-as-judge evaluation.

Usage:
    python3 scripts/eval_similarity.py benches/data/experiments/
"""
from __future__ import annotations

import sys
from pathlib import Path


def main():
    if len(sys.argv) < 2:
        print("usage: eval_similarity.py <experiments_dir>", file=sys.stderr)
        sys.exit(1)

    experiments_dir = Path(sys.argv[1])

    # Find experiments with both base and AAP turn-0 outputs
    pairs = []
    for exp_dir in sorted(experiments_dir.iterdir()):
        if not exp_dir.is_dir():
            continue
        base_outputs = list((exp_dir / "outputs" / "base").glob("turn-0.*"))
        aap_outputs = list((exp_dir / "outputs" / "aap").glob("turn-0.*"))
        if base_outputs and aap_outputs:
            pairs.append((base_outputs[0], aap_outputs[0]))

    if not pairs:
        print("No paired outputs found. Run experiments first.", file=sys.stderr)
        sys.exit(1)

    # TODO: compute similarity metrics
    print(f"Found {len(pairs)} artifact pairs to compare")


if __name__ == "__main__":
    main()
