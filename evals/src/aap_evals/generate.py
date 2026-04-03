"""Generate experiment input directories from the prompt catalog (no LLM needed)."""

from __future__ import annotations

from pathlib import Path

from .markers import marker_example
from .types import Prompt


def default_system_prompt(fmt: str) -> str:
    return f"You produce {fmt} artifacts. Output raw code only. No markdown fences, no explanation."


def aap_init_system_prompt(fmt: str) -> str:
    example = marker_example(fmt)
    return (
        f"You produce {fmt} artifacts with AAP section markers for incremental updates.\n\n"
        f"Wrap each major block with section markers: {example}\n\n"
        "Output raw code only. No markdown fences, no explanation."
    )


def aap_maintain_system_prompt(fmt: str) -> str:
    return f"""You are an AAP maintain-agent. Given an artifact and an edit instruction, use the
provided tool calls to apply changes.

The artifact format is {fmt}.

Choose diff_replace for small text changes (updating a number, changing a word).
Choose section_update for rewriting an entire section.

You may call multiple tools in sequence. After all edits, return a short confirmation."""


def generate_experiment(p: Prompt, exp_dir: Path) -> None:
    """Create the input directory structure for one experiment."""
    base_dir = exp_dir / "inputs" / "base"
    aap_dir = exp_dir / "inputs" / "aap"

    base_sys = default_system_prompt(p.format)
    _write(base_dir / "system.md", base_sys)
    _write(base_dir / "turn-0.md", p.prompt)

    for i, turn in enumerate(p.turns):
        _write(base_dir / f"turn-{i + 1}.md", turn)

    init_sys = aap_init_system_prompt(p.format)
    maintain_sys = aap_maintain_system_prompt(p.format)
    _write(aap_dir / "init-system.md", init_sys)
    _write(aap_dir / "maintain-system.md", maintain_sys)

    (exp_dir / "outputs" / "base").mkdir(parents=True, exist_ok=True)
    (exp_dir / "outputs" / "aap").mkdir(parents=True, exist_ok=True)

    turns_table = "\n".join(
        f"| {i + 1} | {t[:77] + '...' if len(t) > 80 else t} |"
        for i, t in enumerate(p.turns)
    )
    readme = f"""# Experiment: {p.id}

**Format:** {p.format} | **Size:** {p.size_hint} | **Edits:** {len(p.turns)}

**Expected sections:** {', '.join(p.expected_sections)}

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | {len(base_sys)} | {len(base_sys) // 4} |
| AAP init system | {len(init_sys)} | {len(init_sys) // 4} |
| AAP maintain system | {len(maintain_sys)} | {len(maintain_sys) // 4} |
| **Protocol overhead** | | **~{(len(init_sys) + len(maintain_sys) - len(base_sys)) // 4} tokens** |

## Turns

| Turn | Edit |
|---|---|
| 0 | (creation) |
{turns_table}"""
    _write(exp_dir / "README.md", readme)


def generate_all(prompts: list[Prompt], base_dir: Path, count: int) -> int:
    """Create experiment directories for all prompts. Returns count generated."""
    base_dir.mkdir(parents=True, exist_ok=True)
    n = min(count, len(prompts)) if count > 0 else len(prompts)

    for i in range(n):
        p = prompts[i % len(prompts)]
        num = f"{i + 1:03d}"
        exp_dir = base_dir / f"{num}-{p.id}"
        generate_experiment(p, exp_dir)

    return n


def _write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
