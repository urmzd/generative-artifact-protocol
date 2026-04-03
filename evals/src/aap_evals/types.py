"""Data types for AAP benchmark experiments."""

from __future__ import annotations

from pydantic import BaseModel, Field


class Prompt(BaseModel):
    id: str
    format: str
    extension: str
    filename: str
    size_hint: str
    expected_sections: list[str]
    prompt: str
    turns: list[str]


class TurnMetrics(BaseModel):
    turn: int
    edit: str = ""
    input_tokens: int = 0
    output_tokens: int = 0
    latency_ms: int = 0
    output_bytes: int = 0
    envelope_parsed: bool = False
    apply_succeeded: bool = False
    apply_latency_us: int = 0
    envelope_name: str = ""
    envelope_ops_count: int = 0


class FlowData(BaseModel):
    system_prompt_tokens: int = 0
    per_turn: list[TurnMetrics] = Field(default_factory=list)
    total_input_tokens: int = 0
    total_output_tokens: int = 0
    total_latency_ms: int = 0

    def summarize(self) -> None:
        self.total_input_tokens = sum(m.input_tokens for m in self.per_turn)
        self.total_output_tokens = sum(m.output_tokens for m in self.per_turn)
        self.total_latency_ms = sum(m.latency_ms for m in self.per_turn)


class AAPData(FlowData):
    maintain_system_prompt_tokens: int = 0
    envelope_parse_rate: float = 0.0
    apply_success_rate: float = 0.0

    def summarize(self) -> None:
        super().summarize()
        edit_turns = [m for m in self.per_turn if m.turn > 0]
        if edit_turns:
            self.envelope_parse_rate = (
                sum(1 for m in edit_turns if m.envelope_parsed) / len(edit_turns)
            )
            self.apply_success_rate = (
                sum(1 for m in edit_turns if m.apply_succeeded) / len(edit_turns)
            )


class Comparison(BaseModel):
    output_token_savings_pct: float = 0.0
    input_token_savings_pct: float = 0.0
    latency_savings_pct: float = 0.0
    break_even_turn: int = 0
    protocol_overhead_tokens: int = 0


class Experiment(BaseModel):
    experiment_id: str
    prompt_id: str
    model: str
    provider: str
    timestamp: str = ""
    default_flow: FlowData = Field(default_factory=FlowData)
    aap_flow: AAPData = Field(default_factory=AAPData)
    comparison: Comparison | None = None

    def compute_comparison(self) -> None:
        d = self.default_flow
        a = self.aap_flow
        if not d.per_turn or not a.per_turn:
            return
        c = Comparison(
            protocol_overhead_tokens=a.system_prompt_tokens - d.system_prompt_tokens,
        )
        if d.total_output_tokens > 0:
            c.output_token_savings_pct = (
                100 * (d.total_output_tokens - a.total_output_tokens) / d.total_output_tokens
            )
        if d.total_input_tokens > 0:
            c.input_token_savings_pct = (
                100 * (d.total_input_tokens - a.total_input_tokens) / d.total_input_tokens
            )
        if d.total_latency_ms > 0:
            c.latency_savings_pct = (
                100 * (d.total_latency_ms - a.total_latency_ms) / d.total_latency_ms
            )

        def_cum = 0
        aap_cum = 0
        for t in range(min(len(d.per_turn), len(a.per_turn))):
            def_cum += d.per_turn[t].output_tokens
            aap_cum += a.per_turn[t].output_tokens
            if aap_cum < def_cum and c.break_even_turn == 0:
                c.break_even_turn = t

        self.comparison = c
