"""AAP structured output types for the maintain-agent.

Uses Pydantic AI's native structured output (`output_type`) to get
the LLM to return a validated AAP envelope via the OpenAI-compatible
structured output API.
"""

from __future__ import annotations

from pydantic import BaseModel, Field


class DiffTarget(BaseModel):
    search: str = Field(description="Exact substring to find in the artifact. Must be verbatim.")


class DiffOp(BaseModel):
    op: str = Field(description="Operation type: replace, delete, insert_before, or insert_after")
    target: DiffTarget
    content: str = Field(default="", description="Replacement or insertion text")


class SectionOp(BaseModel):
    id: str = Field(description="Section identifier (e.g. 'nav', 'stats')")
    content: str = Field(description="New content for the section")


class EnvelopeResponse(BaseModel):
    """AAP envelope returned by the maintain-agent via structured output."""

    name: str = Field(description="Operation type: 'diff' or 'section'")
    content: list[dict] = Field(description="Array of operation objects matching the chosen name")
