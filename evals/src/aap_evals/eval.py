"""Evaluation reports — cost, reliability, similarity."""

from __future__ import annotations

import difflib
import json
from pathlib import Path

from rich.console import Console
from rich.table import Table

from .types import Experiment

console = Console()


def _load_experiments(exp_dir: Path) -> list[Experiment]:
    results = []
    for d in sorted(exp_dir.iterdir()):
        metrics = d / "outputs" / "metrics.json"
        if metrics.exists():
            results.append(Experiment.model_validate_json(metrics.read_text()))
    return results


# ── Cost ─────────────────────────────────────────────────────────────────


def print_cost_report(exp_dir: Path) -> None:
    experiments = _load_experiments(exp_dir)
    if not experiments:
        console.print("[yellow]No metrics.json files found.[/yellow]")
        return

    # Per-turn table
    table = Table(title="Per-Turn Token Comparison")
    table.add_column("Experiment")
    table.add_column("Turn", justify="right")
    table.add_column("Default In", justify="right")
    table.add_column("Default Out", justify="right")
    table.add_column("AAP In", justify="right")
    table.add_column("AAP Out", justify="right")
    table.add_column("Out Savings", justify="right", style="green")

    for exp in experiments:
        d_turns = exp.default_flow.per_turn
        a_turns = exp.aap_flow.per_turn
        for t in range(min(len(d_turns), len(a_turns))):
            d, a = d_turns[t], a_turns[t]
            savings = (
                f"{100 * (d.output_tokens - a.output_tokens) / d.output_tokens:.1f}%"
                if d.output_tokens > 0 else "—"
            )
            table.add_row(
                exp.experiment_id[:25] if t == 0 else "",
                str(t), str(d.input_tokens), str(d.output_tokens),
                str(a.input_tokens), str(a.output_tokens), savings,
            )

    console.print(table)

    # Aggregate
    agg_table = Table(title="Aggregate Summary")
    agg_table.add_column("Metric")
    agg_table.add_column("Value", justify="right")

    savings = [e.comparison.output_token_savings_pct for e in experiments if e.comparison]
    if savings:
        agg_table.add_row("Mean output savings", f"{sum(savings) / len(savings):.1f}%")
        agg_table.add_row("Min output savings", f"{min(savings):.1f}%")
        agg_table.add_row("Max output savings", f"{max(savings):.1f}%")

    break_evens = [e.comparison.break_even_turn for e in experiments if e.comparison and e.comparison.break_even_turn > 0]
    if break_evens:
        agg_table.add_row("Mean break-even turn", f"{sum(break_evens) / len(break_evens):.1f}")

    console.print()
    console.print(agg_table)


# ── Reliability ──────────────────────────────────────────────────────────


def print_reliability_report(exp_dir: Path) -> None:
    experiments = _load_experiments(exp_dir)
    if not experiments:
        console.print("[yellow]No metrics.json files found.[/yellow]")
        return

    table = Table(title="AAP Reliability")
    table.add_column("Experiment")
    table.add_column("Edit Turns", justify="right")
    table.add_column("Parse Rate", justify="right")
    table.add_column("Apply Rate", justify="right")
    table.add_column("Ops/Turn", justify="right")

    total_parsed = 0
    total_applied = 0
    total_edit_turns = 0

    for exp in experiments:
        edit_turns = [m for m in exp.aap_flow.per_turn if m.turn > 0]
        n = len(edit_turns)
        parsed = sum(1 for m in edit_turns if m.envelope_parsed)
        applied = sum(1 for m in edit_turns if m.apply_succeeded)
        avg_ops = sum(m.envelope_ops_count for m in edit_turns) / n if n > 0 else 0

        total_parsed += parsed
        total_applied += applied
        total_edit_turns += n

        table.add_row(
            exp.experiment_id[:25], str(n),
            f"{parsed / n:.0%}" if n else "—",
            f"{applied / n:.0%}" if n else "—",
            f"{avg_ops:.1f}",
        )

    console.print(table)

    if total_edit_turns > 0:
        console.print(f"\n[bold]Overall:[/bold] {total_parsed}/{total_edit_turns} parsed "
                       f"({total_parsed / total_edit_turns:.0%}), "
                       f"{total_applied}/{total_edit_turns} applied "
                       f"({total_applied / total_edit_turns:.0%})")

    # Breakdown by operation name
    name_stats: dict[str, dict[str, int]] = {}
    for exp in experiments:
        for m in exp.aap_flow.per_turn:
            if m.turn > 0 and m.envelope_name:
                stats = name_stats.setdefault(m.envelope_name, {"total": 0, "succeeded": 0})
                stats["total"] += 1
                if m.apply_succeeded:
                    stats["succeeded"] += 1

    if name_stats:
        op_table = Table(title="By Operation Type")
        op_table.add_column("Operation")
        op_table.add_column("Count", justify="right")
        op_table.add_column("Success Rate", justify="right")
        for name, stats in sorted(name_stats.items()):
            op_table.add_row(
                name, str(stats["total"]),
                f"{stats['succeeded'] / stats['total']:.0%}" if stats["total"] else "—",
            )
        console.print()
        console.print(op_table)


# ── Similarity ───────────────────────────────────────────────────────────


def print_similarity_report(exp_dir: Path) -> None:
    experiments = _load_experiments(exp_dir)
    if not experiments:
        console.print("[yellow]No metrics.json files found.[/yellow]")
        return

    table = Table(title="Artifact Similarity (turn-0)")
    table.add_column("Experiment")
    table.add_column("Ratio", justify="right")
    table.add_column("Additions", justify="right")
    table.add_column("Deletions", justify="right")

    for exp in experiments:
        exp_path = exp_dir / exp.experiment_id

        # Find turn-0 outputs
        base_files = list((exp_path / "outputs" / "base").glob("turn-0.*"))
        aap_files = list((exp_path / "outputs" / "aap").glob("turn-0.*"))

        if not base_files or not aap_files:
            table.add_row(exp.experiment_id[:25], "—", "—", "—")
            continue

        base_text = base_files[0].read_text()
        aap_text = aap_files[0].read_text()

        ratio = difflib.SequenceMatcher(None, base_text, aap_text).ratio()

        diff = list(difflib.unified_diff(base_text.splitlines(), aap_text.splitlines()))
        additions = sum(1 for l in diff if l.startswith("+") and not l.startswith("+++"))
        deletions = sum(1 for l in diff if l.startswith("-") and not l.startswith("---"))

        table.add_row(
            exp.experiment_id[:25],
            f"{ratio:.3f}",
            str(additions),
            str(deletions),
        )

    console.print(table)
