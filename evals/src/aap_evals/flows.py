"""Experiment flow execution — runs baseline and AAP flows via Pydantic AI + Ollama.

System prompts are read from disk (inputs/base/ and inputs/aap/), not auto-generated.
The AAP maintain-agent uses native structured output (output_type) to return envelopes.
"""

from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path

from pydantic_ai import Agent
from pydantic_ai.models.openai import OpenAIChatModel
from pydantic_ai.providers.ollama import OllamaProvider
from pydantic_ai.providers.openai import OpenAIProvider
from rich.console import Console

from .apply import apply_envelope
from .tools import EnvelopeResponse
from .types import AAPData, Experiment, FlowData, Prompt, TurnMetrics

console = Console()


def create_model(provider: str, model_name: str, host: str) -> OpenAIChatModel:
    """Create a Pydantic AI model for the given provider."""
    if provider == "ollama":
        base = host.rstrip("/")
        if not base.endswith("/v1"):
            base += "/v1"
        return OpenAIChatModel(
            model_name=model_name or "qwen3.5:4b",
            provider=OllamaProvider(base_url=base),
        )
    elif provider == "openai":
        return OpenAIChatModel(
            model_name=model_name or "gpt-4o-mini",
            provider=OpenAIProvider(),
        )
    else:
        raise ValueError(f"unsupported provider: {provider}")


def run_experiment(
    exp_dir: Path,
    prompt_meta: Prompt,
    model: OpenAIChatModel,
    model_name: str,
    provider_name: str,
    verbose: bool = False,
) -> Experiment:
    """Run both flows for a single experiment and return metrics."""
    base_dir = exp_dir / "inputs" / "base"
    aap_dir = exp_dir / "inputs" / "aap"
    out_base = exp_dir / "outputs" / "base"
    out_aap = exp_dir / "outputs" / "aap"
    out_base.mkdir(parents=True, exist_ok=True)
    out_aap.mkdir(parents=True, exist_ok=True)

    # Read system prompts from disk
    base_system = (base_dir / "system.md").read_text()
    aap_init_system = (aap_dir / "init-system.md").read_text()
    aap_maintain_system = (aap_dir / "maintain-system.md").read_text()

    # Read turn files
    turns: list[str] = []
    i = 0
    while (base_dir / f"turn-{i}.md").exists():
        turns.append((base_dir / f"turn-{i}.md").read_text())
        i += 1

    ext = prompt_meta.extension
    fmt = prompt_meta.format

    experiment = Experiment(
        experiment_id=exp_dir.name,
        prompt_id=prompt_meta.id,
        model=model_name,
        provider=provider_name,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )

    # ── Baseline flow ────────────────────────────────────────────────────
    if verbose:
        console.print(f"  [dim]baseline flow ({len(turns)} turns)...[/dim]")

    default_flow = _run_default_flow(model, base_system, turns, prompt_meta, ext, out_base, verbose)
    default_flow.summarize()
    experiment.default_flow = default_flow

    # ── AAP flow ─────────────────────────────────────────────────────────
    if verbose:
        console.print(f"  [dim]AAP flow ({len(turns)} turns)...[/dim]")

    aap_flow = _run_aap_flow(
        model, fmt, aap_init_system, aap_maintain_system,
        turns, prompt_meta, ext, out_aap, verbose,
    )
    aap_flow.summarize()
    experiment.aap_flow = aap_flow

    experiment.compute_comparison()

    (exp_dir / "outputs" / "metrics.json").write_text(
        experiment.model_dump_json(indent=2) + "\n"
    )
    return experiment


def _run_default_flow(
    model: OpenAIChatModel,
    system_prompt: str,
    turns: list[str],
    prompt_meta: Prompt,
    ext: str,
    out_dir: Path,
    verbose: bool,
) -> FlowData:
    """Baseline: growing conversation, full artifact regenerated each turn."""
    flow = FlowData(system_prompt_tokens=len(system_prompt) // 4)
    agent: Agent[None, str] = Agent(model, system_prompt=system_prompt)

    message_history = None

    for i, turn_text in enumerate(turns):
        edit = "(creation)" if i == 0 else (prompt_meta.turns[i - 1][:60] if i - 1 < len(prompt_meta.turns) else "")

        t0 = time.perf_counter()
        if message_history is None:
            result = agent.run_sync(turn_text)
        else:
            result = agent.run_sync(turn_text, message_history=message_history)
        latency_ms = int((time.perf_counter() - t0) * 1000)

        artifact = result.output
        usage = result.usage()
        message_history = result.all_messages()

        flow.per_turn.append(TurnMetrics(
            turn=i, edit=edit,
            input_tokens=usage.input_tokens,
            output_tokens=usage.output_tokens,
            latency_ms=latency_ms,
            output_bytes=len(artifact.encode()),
        ))
        (out_dir / f"turn-{i}{ext}").write_text(artifact)

        if verbose:
            console.print(f"    turn {i}: {usage.input_tokens} in / {usage.output_tokens} out / {latency_ms}ms")

    _save_conversation(out_dir / "conversation.json", message_history or [])
    return flow


def _run_aap_flow(
    model: OpenAIChatModel,
    fmt: str,
    init_system: str,
    maintain_system: str,
    turns: list[str],
    prompt_meta: Prompt,
    ext: str,
    out_dir: Path,
    verbose: bool,
) -> AAPData:
    """AAP: structured-output envelopes with bounded context per turn."""
    flow = AAPData()
    flow.system_prompt_tokens = len(init_system) // 4
    flow.maintain_system_prompt_tokens = len(maintain_system) // 4

    # ── Turn 0: init-agent creates artifact with section markers ─────────
    init_agent: Agent[None, str] = Agent(model, system_prompt=init_system)

    t0 = time.perf_counter()
    result = init_agent.run_sync(turns[0])
    latency_ms = int((time.perf_counter() - t0) * 1000)

    artifact = result.output
    usage = result.usage()

    flow.per_turn.append(TurnMetrics(
        turn=0, edit="(creation)",
        input_tokens=usage.input_tokens,
        output_tokens=usage.output_tokens,
        latency_ms=latency_ms,
        output_bytes=len(artifact.encode()),
    ))
    (out_dir / f"turn-0{ext}").write_text(artifact)

    if verbose:
        console.print(f"    turn 0 (init): {usage.input_tokens} in / {usage.output_tokens} out / {latency_ms}ms")

    # ── Turns 1..N: maintain-agent with structured output ────────────────
    maintain_agent: Agent[None, EnvelopeResponse] = Agent(
        model,
        system_prompt=maintain_system,
        output_type=EnvelopeResponse,
    )

    for i in range(1, len(turns)):
        edit = prompt_meta.turns[i - 1][:60] if i - 1 < len(prompt_meta.turns) else ""

        user_msg = f"## Current Artifact\n\n```\n{artifact}\n```\n\n## Edit Instruction\n\n{turns[i]}"

        t0 = time.perf_counter()
        parsed = False
        succeeded = False
        envelope_name = ""
        ops_count = 0

        try:
            result = maintain_agent.run_sync(user_msg)
            latency_ms = int((time.perf_counter() - t0) * 1000)
            usage = result.usage()

            envelope: EnvelopeResponse = result.output
            parsed = True
            envelope_name = envelope.name
            ops_count = len(envelope.content)

            # Apply the envelope
            t_apply = time.perf_counter()
            new_artifact = apply_envelope(artifact, envelope.name, envelope.content, fmt)
            apply_us = int((time.perf_counter() - t_apply) * 1_000_000)

            succeeded = True
            artifact = new_artifact

            # Write raw envelope
            (out_dir / f"turn-{i}.json").write_text(
                envelope.model_dump_json(indent=2) + "\n"
            )

        except Exception as e:
            latency_ms = int((time.perf_counter() - t0) * 1000)
            apply_us = 0
            usage = type("U", (), {"input_tokens": 0, "output_tokens": 0})()
            if verbose:
                console.print(f"    turn {i}: [red]error: {e}[/red]")

        flow.per_turn.append(TurnMetrics(
            turn=i, edit=edit,
            input_tokens=usage.input_tokens,
            output_tokens=usage.output_tokens,
            latency_ms=latency_ms,
            output_bytes=len(artifact.encode()),
            envelope_parsed=parsed,
            apply_succeeded=succeeded,
            apply_latency_us=apply_us,
            envelope_name=envelope_name,
            envelope_ops_count=ops_count,
        ))
        (out_dir / f"turn-{i}{ext}").write_text(artifact)

        if verbose:
            tag = "[green]ok[/green]" if succeeded else "[red]fail[/red]"
            console.print(
                f"    turn {i}: {usage.input_tokens} in / {usage.output_tokens} out / "
                f"{latency_ms}ms / {ops_count} ops / {tag}"
            )

    _save_conversation(out_dir / "conversation.json", [])
    return flow


def _save_conversation(path: Path, messages: list) -> None:
    try:
        path.write_text(json.dumps([str(m) for m in messages], indent=2, default=str) + "\n")
    except Exception:
        path.write_text("[]\n")
