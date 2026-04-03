"""Artifact generation via pydantic-ai + Ollama."""

from __future__ import annotations

import re

from pydantic_ai import Agent
from pydantic_ai.models.openai import OpenAIChatModel
from pydantic_ai.providers.ollama import OllamaProvider

from .categories import Category
from .markers import marker_example


def build_prompt(cat: Category, variant_idx: int) -> str:
    variant = cat.variants[variant_idx % len(cat.variants)]
    me = marker_example(cat.fmt)

    sections_instruction = ""
    if cat.sections and me:
        section_list = ", ".join(cat.sections)
        sections_instruction = (
            f"\nWrap each major section with markers using EXACTLY this syntax: {me}\n"
            f"Replace ID with the section name.\n\n"
            f"You MUST include these section IDs: {section_list}\n"
        )

    return (
        f"{cat.prompt_base} {variant}.\n\n"
        f"Requirements:\n"
        f"- Self-contained, realistic, production-quality code/content\n"
        f"- At least 80 lines of meaningful content\n"
        f"- Use diverse, realistic data values (names, numbers, strings)\n"
        f"{sections_instruction}\n"
        f"Output ONLY the raw {cat.ext} content. No markdown fences, no explanation, no commentary."
    )


def clean_artifact(text: str) -> str:
    text = text.strip()
    if text.startswith("```"):
        nl = text.find("\n")
        if nl != -1:
            text = text[nl + 1 :]
    if text.endswith("```"):
        text = text[:-3].rstrip()
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()
    return text


def create_generator(model_name: str, host: str) -> Agent[None, str]:
    """Create a pydantic-ai Agent for artifact generation."""
    # OllamaProvider needs /v1 suffix for OpenAI-compatible endpoint
    base = host.rstrip("/")
    if not base.endswith("/v1"):
        base += "/v1"
    model = OpenAIChatModel(
        model_name=model_name,
        provider=OllamaProvider(base_url=base),
    )
    return Agent(
        model,
        system_prompt="You are a code generator. Output only raw code/content. No markdown fences, no explanation.",
    )


def generate_artifact(
    agent: Agent[None, str],
    cat: Category,
    variant_idx: int,
) -> str:
    """Generate a single artifact using the pydantic-ai agent."""
    prompt = build_prompt(cat, variant_idx)
    result = agent.run_sync(prompt)
    return clean_artifact(result.output)
