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

DATA_DIR = Path(__file__).resolve().parent.parent.parent.parent / "benches" / "data"


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
