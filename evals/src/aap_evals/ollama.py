"""Async Ollama client for artifact generation."""

from __future__ import annotations

import re

import aiohttp

from .categories import Category
from .markers import marker_example

OLLAMA_URL = "http://localhost:11434/api/generate"
DEFAULT_MODEL = "gemma4"
DEFAULT_TIMEOUT = 180


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


async def generate_artifact(
    session: aiohttp.ClientSession,
    cat: Category,
    variant_idx: int,
    sem: "asyncio.Semaphore",
    model: str = DEFAULT_MODEL,
) -> str:
    import asyncio

    prompt = build_prompt(cat, variant_idx)

    for attempt in range(3):
        try:
            async with sem:
                async with session.post(
                    OLLAMA_URL,
                    json={
                        "model": model,
                        "prompt": prompt,
                        "stream": False,
                        "options": {"temperature": 0.7, "num_predict": 4096},
                    },
                    timeout=aiohttp.ClientTimeout(total=DEFAULT_TIMEOUT),
                ) as resp:
                    if resp.status != 200:
                        body = await resp.text()
                        raise RuntimeError(f"Ollama {resp.status}: {body[:200]}")
                    data = await resp.json()
                    text = clean_artifact(data.get("response", ""))
                    if len(text) < 50:
                        raise RuntimeError("artifact too short")
                    return text
        except Exception as e:
            if attempt == 2:
                raise
            await asyncio.sleep(2**attempt)

    raise RuntimeError("unreachable")
