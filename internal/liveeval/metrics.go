package liveeval

type Metrics struct {
	ExperimentID  string          `json:"experiment_id"`
	Model         string          `json:"model"`
	Provider      string          `json:"provider"`
	Timestamp     string          `json:"timestamp"`
	Format        string          `json:"format"`
	BaseTurn0     *TurnMetrics    `json:"base_turn0,omitempty"`
	GAPTurn0      *TurnMetrics    `json:"gap_turn0,omitempty"`
	StatelessT0   *TurnMetrics    `json:"stateless_turn0,omitempty"`
	StatelessFlow *FlowMetrics    `json:"stateless_flow,omitempty"`
	DefaultFlow   *FlowMetrics    `json:"default_flow,omitempty"`
	GAPFlow       *GapFlowMetrics `json:"gap_flow,omitempty"`
	Comparison    *Comparison     `json:"comparison,omitempty"`
	Decomposition *Decomposition  `json:"decomposition,omitempty"`
	Reliability   *Reliability    `json:"reliability,omitempty"`
	Economics     *Economics      `json:"economics,omitempty"`
	Validity      *Validity       `json:"validity,omitempty"`
}

type TurnMetrics struct {
	InputTokens       uint64 `json:"input_tokens"`
	OutputTokens      uint64 `json:"output_tokens"`
	CachedInputTokens uint64 `json:"cached_input_tokens,omitempty"`
	LatencyMS         uint64 `json:"latency_ms"`
	ArtifactBytes     int    `json:"artifact_bytes"`
}

type TurnResult struct {
	Turn              int     `json:"turn"`
	Edit              string  `json:"edit"`
	InputTokens       uint64  `json:"input_tokens"`
	OutputTokens      uint64  `json:"output_tokens"`
	CachedInputTokens uint64  `json:"cached_input_tokens,omitempty"`
	LatencyMS         uint64  `json:"latency_ms"`
	OutputBytes       int     `json:"output_bytes"`
	Retried           *bool   `json:"retried,omitempty"`
	Failed            bool    `json:"failed"`
	FailureReason     *string `json:"failure_reason,omitempty"`
	EnvelopeParsed    *bool   `json:"envelope_parsed,omitempty"`
	ApplySucceeded    *bool   `json:"apply_succeeded,omitempty"`
	EnvelopeName      *string `json:"envelope_name,omitempty"`
	RepairAttempts    int     `json:"repair_attempts,omitempty"`
	ValidationError   *string `json:"validation_error,omitempty"`
}

type FlowMetrics struct {
	PerTurn            []TurnResult `json:"per_turn"`
	TotalInputTokens   uint64       `json:"total_input_tokens"`
	TotalOutputTokens  uint64       `json:"total_output_tokens"`
	TotalCachedInput   uint64       `json:"total_cached_input_tokens,omitempty"`
	TotalLatencyMillis uint64       `json:"total_latency_ms"`
}

type GapFlowMetrics struct {
	FlowMetrics
	EnvelopeParseRate float64 `json:"envelope_parse_rate"`
	ApplySuccessRate  float64 `json:"apply_success_rate"`
}

type Comparison struct {
	OutputTokenSavingsPct float64 `json:"output_token_savings_pct"`
	InputTokenSavingsPct  float64 `json:"input_token_savings_pct"`
	LatencySavingsPct     float64 `json:"latency_savings_pct"`
}

type Decomposition struct {
	InputSavingsBVsAPct  float64 `json:"input_savings_b_vs_a_pct"`
	OutputSavingsCVsBPct float64 `json:"output_savings_c_vs_b_pct"`
	InputSavingsCVsAPct  float64 `json:"input_savings_c_vs_a_pct"`
	OutputSavingsCVsAPct float64 `json:"output_savings_c_vs_a_pct"`
}

type Reliability struct {
	EditTurns            int            `json:"edit_turns"`
	MissCount            int            `json:"miss_count"`
	MissRate             float64        `json:"miss_rate"`
	ParseMissCount       int            `json:"parse_miss_count"`
	ValidationMissCount  int            `json:"validation_miss_count"`
	InvalidEnvelopeCount int            `json:"invalid_envelope_count"`
	ApplyMissCount       int            `json:"apply_miss_count"`
	RequestFailureCount  int            `json:"request_failure_count"`
	UnknownMissCount     int            `json:"unknown_miss_count"`
	ByReason             map[string]int `json:"by_reason,omitempty"`
}

type Economics struct {
	FallbackAssumption           string              `json:"fallback_assumption"`
	MeasuredTotalTokenSavingsPct float64             `json:"measured_total_token_savings_pct"`
	FallbackAdjusted             *FallbackAdjusted   `json:"fallback_adjusted,omitempty"`
	Amortized                    *AmortizedEconomics `json:"amortized,omitempty"`
}

type FallbackAdjusted struct {
	InputTokens               uint64  `json:"input_tokens"`
	OutputTokens              uint64  `json:"output_tokens"`
	TotalTokens               uint64  `json:"total_tokens"`
	MissAttemptInputTokens    uint64  `json:"miss_attempt_input_tokens"`
	MissAttemptOutputTokens   uint64  `json:"miss_attempt_output_tokens"`
	MissAttemptTotalTokens    uint64  `json:"miss_attempt_total_tokens"`
	FallbackRetryInputTokens  uint64  `json:"fallback_retry_input_tokens"`
	FallbackRetryOutputTokens uint64  `json:"fallback_retry_output_tokens"`
	FallbackRetryTotalTokens  uint64  `json:"fallback_retry_total_tokens"`
	InputTokenSavingsPct      float64 `json:"input_token_savings_pct"`
	OutputTokenSavingsPct     float64 `json:"output_token_savings_pct"`
	TotalTokenSavingsPct      float64 `json:"total_token_savings_pct"`
}

type AmortizedEconomics struct {
	EditTurns                            int     `json:"edit_turns"`
	BaseInitInclusiveTokens              uint64  `json:"base_init_inclusive_tokens"`
	GAPInitInclusiveTokens               uint64  `json:"gap_init_inclusive_tokens"`
	FallbackInitInclusiveTokens          uint64  `json:"fallback_init_inclusive_tokens"`
	MeasuredInitInclusiveTokenSavingsPct float64 `json:"measured_init_inclusive_token_savings_pct"`
	FallbackInitInclusiveTokenSavingsPct float64 `json:"fallback_init_inclusive_token_savings_pct"`
}

type Validity struct {
	GAPRunDegenerate  bool `json:"gap_run_degenerate"`
	BaseInputMonotone bool `json:"base_input_monotone"`
}

func toFlowMetrics(turns []TurnResult) FlowMetrics {
	var flow FlowMetrics
	flow.PerTurn = turns
	for _, turn := range turns {
		flow.TotalInputTokens += turn.InputTokens
		flow.TotalOutputTokens += turn.OutputTokens
		flow.TotalCachedInput += turn.CachedInputTokens
		flow.TotalLatencyMillis += turn.LatencyMS
	}
	return flow
}
