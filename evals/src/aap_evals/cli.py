"""CLI entry point — typer + rich."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Annotated, Optional

import typer
from rich.console import Console
from rich.table import Table

from .types import Prompt

app = typer.Typer(name="aap-evals", help="AAP protocol benchmarks and evaluations.")
console = Console()

DATA_DIR = Path(__file__).resolve().parent.parent.parent / "data"


def _load_prompts(path: Path) -> list[Prompt]:
    data = json.loads(path.read_text())
    return [Prompt(**p) for p in data]


def _find_experiments(exp_dir: Path) -> list[Path]:
    """List experiment directories sorted by number."""
    dirs = [d for d in exp_dir.iterdir() if d.is_dir() and d.name[0].isdigit()]
    return sorted(dirs)


# ── generate ─────────────────────────────────────────────────────────────


@app.command()
def generate(
    prompts: Annotated[Path, typer.Option(help="Path to prompts.json")] = DATA_DIR / "prompts.json",
    output: Annotated[Path, typer.Option(help="Output directory")] = DATA_DIR / "experiments",
    count: Annotated[int, typer.Option(help="Number of experiments (0 = all)")] = 0,
) -> None:
    """Generate experiment input directories from the prompt catalog (no LLM)."""
    from .generate import generate_all

    prompt_list = _load_prompts(prompts)
    n = generate_all(prompt_list, output, count)
    console.print(f"[green]Generated {n} experiment directories in {output}[/green]")


# ── run ──────────────────────────────────────────────────────────────────


@app.command()
def run(
    experiments: Annotated[Path, typer.Option(help="Experiments directory")] = DATA_DIR / "experiments",
    prompts: Annotated[Path, typer.Option(help="Path to prompts.json")] = DATA_DIR / "prompts.json",
    provider: Annotated[str, typer.Option(help="LLM provider")] = "ollama",
    model: Annotated[str, typer.Option(help="Model name")] = "",
    host: Annotated[str, typer.Option(help="Ollama host")] = "http://localhost:11434",
    single: Annotated[int, typer.Option(help="Run single experiment by number")] = 0,
    count: Annotated[int, typer.Option(help="Number of experiments to run")] = 0,
    verbose: Annotated[bool, typer.Option(help="Verbose output")] = False,
) -> None:
    """Execute experiments against an LLM and collect metrics."""
    from .flows import create_model, run_experiment

    prompt_list = _load_prompts(prompts)
    prompt_map = {p.id: p for p in prompt_list}

    llm_model = create_model(provider, model, host)
    model_name = model or ("qwen3.5:4b" if provider == "ollama" else "gpt-4o-mini")

    exp_dirs = _find_experiments(experiments)

    if single > 0:
        exp_dirs = [d for d in exp_dirs if d.name.startswith(f"{single:03d}-")]
    elif count > 0:
        exp_dirs = exp_dirs[:count]

    if not exp_dirs:
        console.print("[red]No experiments found.[/red]")
        raise typer.Exit(1)

    console.print(f"Running {len(exp_dirs)} experiment(s) with [bold]{model_name}[/bold] via {provider}\n")

    results = []
    for exp_dir in exp_dirs:
        # Extract prompt ID from directory name (e.g. "001-html-dashboard-ecommerce")
        prompt_id = "-".join(exp_dir.name.split("-")[1:])
        prompt_meta = prompt_map.get(prompt_id)
        if not prompt_meta:
            console.print(f"[yellow]Skipping {exp_dir.name}: prompt {prompt_id!r} not in catalog[/yellow]")
            continue

        console.print(f"[bold]{exp_dir.name}[/bold]")
        result = run_experiment(exp_dir, prompt_meta, llm_model, model_name, provider, verbose)
        results.append(result)

        c = result.comparison
        if c:
            console.print(
                f"  output savings: [green]{c.output_token_savings_pct:.1f}%[/green] | "
                f"break-even: turn {c.break_even_turn}\n"
            )

    # Summary table
    if results:
        _print_summary(results)


def _print_summary(results: list) -> None:
    table = Table(title="Experiment Summary")
    table.add_column("Experiment", style="bold")
    table.add_column("Turns", justify="right")
    table.add_column("Default Out", justify="right")
    table.add_column("AAP Out", justify="right")
    table.add_column("Savings", justify="right", style="green")
    table.add_column("Parse Rate", justify="right")
    table.add_column("Apply Rate", justify="right")

    for r in results:
        c = r.comparison
        table.add_row(
            r.experiment_id[:30],
            str(len(r.default_flow.per_turn)),
            str(r.default_flow.total_output_tokens),
            str(r.aap_flow.total_output_tokens),
            f"{c.output_token_savings_pct:.1f}%" if c else "—",
            f"{r.aap_flow.envelope_parse_rate:.0%}",
            f"{r.aap_flow.apply_success_rate:.0%}",
        )

    console.print()
    console.print(table)


# ── generate-corpus ─────────────────────────────────────────────────────


@app.command()
def generate_corpus(
    output: Annotated[Path, typer.Option(help="Output directory")] = DATA_DIR / "apply-engine",
    model: Annotated[str, typer.Option(help="Ollama model")] = "gemma4",
    host: Annotated[str, typer.Option(help="Ollama host")] = "http://localhost:11434",
    count: Annotated[int, typer.Option(help="Number of test cases (0 = all)")] = 0,
    resume: Annotated[bool, typer.Option(help="Skip existing cases")] = False,
) -> None:
    """Generate apply-engine benchmark corpus — artifacts via Ollama + deterministic envelopes."""
    from .categories import CATEGORIES
    from .envelopes import generate_all_envelopes
    from .markers import extract_section_content
    from .ollama import clean_artifact, create_generator, generate_artifact

    agent = create_generator(model, host)

    # Build flat task list
    tasks: list[tuple] = []
    case_num = 1
    for cat in CATEGORIES:
        for vi in range(cat.count):
            tasks.append((cat, vi, case_num))
            case_num += 1

    if count > 0:
        tasks = tasks[:count]

    total = len(tasks)
    console.print(f"Generating {total} test cases -> {output}/")
    console.print(f"Model: [bold]{model}[/bold]\n")

    output.mkdir(parents=True, exist_ok=True)
    succeeded = 0
    failed = 0

    for cat, vi, cn in tasks:
        case_dir = output / f"{cn:04d}"
        meta_path = case_dir / "meta.json"

        if resume and meta_path.exists():
            succeeded += 1
            continue

        artifact_id = f"artifact-{cn:04d}"

        try:
            content = generate_artifact(agent, cat, vi)
            if len(content) < 50:
                raise RuntimeError("artifact too short")
        except Exception as e:
            console.print(f"  [red]FAIL {cn:04d} ({cat.name}): {e}[/red]")
            failed += 1
            continue

        # Write artifact
        artifacts_dir = case_dir / "artifacts"
        artifacts_dir.mkdir(parents=True, exist_ok=True)
        (artifacts_dir / cat.filename).write_text(content)

        # Generate and write envelopes
        all_envs = generate_all_envelopes(content, artifact_id, cat.fmt, cat.sections)
        envelopes_dir = case_dir / "envelopes"
        envelopes_dir.mkdir(parents=True, exist_ok=True)
        for filename, envs in all_envs.items():
            with open(envelopes_dir / filename, "w") as f:
                for env in envs:
                    f.write(json.dumps(env, separators=(",", ":")) + "\n")

        # Metadata
        valid_sections = [
            sid for sid in cat.sections
            if extract_section_content(content, sid, cat.fmt) is not None
        ]
        meta = {
            "case_num": cn,
            "category": cat.name,
            "format": cat.fmt,
            "extension": cat.ext,
            "filename": cat.filename,
            "variant_index": vi,
            "sections_expected": cat.sections,
            "sections_found": valid_sections,
            "envelope_files": sorted(all_envs.keys()),
            "artifact_bytes": len(content.encode()),
        }
        meta_path.write_text(json.dumps(meta, indent=2) + "\n")

        succeeded += 1
        if succeeded % 10 == 0 or succeeded == total:
            console.print(f"  [{succeeded}/{total}] {succeeded} ok, {failed} failed")

    console.print(f"\n[green]Done: {succeeded}/{total} succeeded, {failed} failed[/green]")


# ── bench ──────────────────────────────────────────────────────────────


@app.command()
def bench(
    corpus: Annotated[Path, typer.Option(help="Apply-engine corpus directory")] = DATA_DIR / "apply-engine",
    output: Annotated[Path, typer.Option(help="Results output path")] = DATA_DIR / "apply-engine" / "results.json",
    count: Annotated[int, typer.Option(help="Max test cases (0 = all)")] = 0,
) -> None:
    """Benchmark the apply engine against the generated corpus."""
    import time
    from statistics import mean, quantiles

    from .apply import apply_envelope

    meta_files = sorted(corpus.glob("*/meta.json"))
    if count > 0:
        meta_files = meta_files[:count]

    if not meta_files:
        console.print("[red]No test cases found. Run generate-corpus first.[/red]")
        raise typer.Exit(1)

    console.print(f"Benchmarking {len(meta_files)} test cases from {corpus}/\n")

    results_by_type: dict[str, list[dict]] = {}

    for mf in meta_files:
        meta = json.loads(mf.read_text())
        case_dir = mf.parent
        artifact_path = case_dir / "artifacts" / meta["filename"]

        if not artifact_path.exists():
            continue

        base_content = artifact_path.read_text()

        for env_file in sorted((case_dir / "envelopes").glob("*.jsonl")):
            env_type = env_file.stem  # e.g. "diff-replace"
            for line in env_file.read_text().strip().split("\n"):
                if not line:
                    continue
                envelope = json.loads(line)
                name = envelope["name"]
                items = envelope["content"]
                fmt = envelope.get("operation", {}).get("format", "text/html")

                t0 = time.perf_counter_ns()
                ok = True
                try:
                    if name == "full":
                        _ = items[0]["body"]
                    elif name == "template":
                        # Template doesn't need base content
                        from .apply import apply_envelope as _apply
                        # Template fill is handled differently — skip for now
                        pass
                    else:
                        apply_envelope(base_content, name, items, fmt)
                except Exception:
                    ok = False
                elapsed_ns = time.perf_counter_ns() - t0

                key = f"{meta['format']}:{env_type}"
                results_by_type.setdefault(key, []).append({
                    "case": meta["case_num"],
                    "elapsed_ns": elapsed_ns,
                    "ok": ok,
                    "artifact_bytes": meta["artifact_bytes"],
                })

    # Aggregate
    summary = []
    for key, entries in sorted(results_by_type.items()):
        fmt, op = key.split(":", 1)
        times = [e["elapsed_ns"] for e in entries]
        ok_count = sum(1 for e in entries if e["ok"])
        qs = quantiles(times, n=100) if len(times) >= 2 else times
        summary.append({
            "format": fmt,
            "operation": op,
            "count": len(entries),
            "success_rate": ok_count / len(entries) if entries else 0,
            "mean_ns": int(mean(times)),
            "p50_ns": int(qs[49]) if len(qs) > 49 else int(mean(times)),
            "p95_ns": int(qs[94]) if len(qs) > 94 else int(max(times)),
            "p99_ns": int(qs[98]) if len(qs) > 98 else int(max(times)),
            "max_ns": int(max(times)),
        })

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2) + "\n")
    console.print(f"[green]Results written to {output}[/green]")

    # Print summary table
    table = Table(title="Apply Engine Benchmark")
    table.add_column("Format", style="bold")
    table.add_column("Operation")
    table.add_column("Count", justify="right")
    table.add_column("Success", justify="right")
    table.add_column("Mean", justify="right")
    table.add_column("P50", justify="right")
    table.add_column("P95", justify="right")

    for s in summary:
        table.add_row(
            s["format"][:20],
            s["operation"],
            str(s["count"]),
            f"{s['success_rate']:.0%}",
            f"{s['mean_ns'] / 1000:.1f}us",
            f"{s['p50_ns'] / 1000:.1f}us",
            f"{s['p95_ns'] / 1000:.1f}us",
        )

    console.print()
    console.print(table)


# ── report ─────────────────────────────────────────────────────────────


@app.command()
def report(
    results_path: Annotated[Path, typer.Option("--input", help="Results JSON")] = DATA_DIR / "apply-engine" / "results.json",
    output: Annotated[Path, typer.Option(help="Markdown output")] = DATA_DIR / "apply-engine" / "results.md",
) -> None:
    """Generate markdown report from bench results."""
    if not results_path.exists():
        console.print("[red]No results.json found. Run bench first.[/red]")
        raise typer.Exit(1)

    summary = json.loads(results_path.read_text())

    lines = ["# Apply Engine Benchmark Results\n"]
    lines.append("| Format | Operation | Count | Success | Mean | P50 | P95 | P99 | Max |")
    lines.append("|--------|-----------|------:|--------:|-----:|----:|----:|----:|----:|")

    for s in summary:
        lines.append(
            f"| {s['format']} | {s['operation']} | {s['count']} | "
            f"{s['success_rate']:.0%} | "
            f"{s['mean_ns']/1000:.1f}us | "
            f"{s['p50_ns']/1000:.1f}us | "
            f"{s['p95_ns']/1000:.1f}us | "
            f"{s['p99_ns']/1000:.1f}us | "
            f"{s['max_ns']/1000:.1f}us |"
        )

    output.write_text("\n".join(lines) + "\n")
    console.print(f"[green]Report written to {output}[/green]")


# ── eval ─────────────────────────────────────────────────────────────────


@app.command()
def eval_cost(
    experiments: Annotated[Path, typer.Option(help="Experiments directory")] = DATA_DIR / "experiments",
) -> None:
    """Analyze token costs across experiments."""
    from .eval import print_cost_report

    print_cost_report(experiments)


@app.command()
def eval_reliability(
    experiments: Annotated[Path, typer.Option(help="Experiments directory")] = DATA_DIR / "experiments",
) -> None:
    """Analyze AAP envelope reliability."""
    from .eval import print_reliability_report

    print_reliability_report(experiments)


@app.command()
def eval_similarity(
    experiments: Annotated[Path, typer.Option(help="Experiments directory")] = DATA_DIR / "experiments",
) -> None:
    """Compare artifacts between baseline and AAP flows."""
    from .eval import print_similarity_report

    print_similarity_report(experiments)
