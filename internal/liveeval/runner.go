package liveeval

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	gap "github.com/urmzd/generative-artifact-protocol"
	"github.com/urmzd/generative-artifact-protocol/evalset"
)

type Config struct {
	ExperimentsDir string
	Count          int
	IDFilter       string
	Flow           string
	Model          string
	APIBase        string
	APIKey         string
	Force          bool
}

var thinkRE = regexp.MustCompile(`(?s)<think>.*?</think>`)

const (
	maxSynthesisAttempts = 2
	maxEditAttempts      = 3
)

func Run(ctx context.Context, cfg Config) error {
	experiments, err := evalset.LoadExperiments(cfg.ExperimentsDir)
	if err != nil {
		return err
	}
	experiments = filterExperiments(experiments, cfg.IDFilter, cfg.Count)
	client := NewClient(cfg.APIBase, cfg.APIKey, cfg.Model)

	for i, exp := range experiments {
		metricsPath := filepath.Join(exp.Paths.ExperimentDir, "metrics.json")
		if !cfg.Force {
			if _, err := os.Stat(metricsPath); err == nil {
				fmt.Fprintf(os.Stderr, "[%d/%d] skip %s (metrics.json exists; use --force)\n", i+1, len(experiments), exp.ExperimentID)
				continue
			}
		}
		fmt.Fprintf(os.Stderr, "[%d/%d] running %s\n", i+1, len(experiments), exp.ExperimentID)
		metrics, err := runExperiment(ctx, client, exp, cfg.Flow)
		if err != nil {
			return fmt.Errorf("%s: %w", exp.ExperimentID, err)
		}
		if err := writeJSON(metricsPath, metrics); err != nil {
			return err
		}
		fmt.Fprintf(os.Stderr, "  wrote %s\n", metricsPath)
	}
	return nil
}

func runExperiment(ctx context.Context, client *Client, exp evalset.ExperimentInput, flow string) (Metrics, error) {
	runBase := flow == "base" || flow == "both" || flow == "abc" || flow == "all"
	runStateless := flow == "stateless" || flow == "abc" || flow == "all"
	runGAP := flow == "gap" || flow == "both" || flow == "abc" || flow == "all"
	if !runBase && !runStateless && !runGAP {
		return Metrics{}, fmt.Errorf("unknown flow %q", flow)
	}

	metrics := Metrics{
		ExperimentID: exp.ExperimentID,
		Model:        client.Model,
		Provider:     "openai-compatible",
		Timestamp:    time.Now().UTC().Format(time.RFC3339),
		Format:       exp.Format,
	}

	var baseArtifact string
	var baseT0 *TurnMetrics
	if runBase {
		fmt.Fprintln(os.Stderr, "  base flow...")
		t0, turns, artifact, err := runBaseFlow(ctx, client, exp)
		if err != nil {
			return Metrics{}, err
		}
		baseArtifact = artifact
		baseT0 = &t0
		flow := toFlowMetrics(turns)
		metrics.BaseTurn0 = &t0
		metrics.DefaultFlow = &flow
	}

	if runStateless {
		fmt.Fprintln(os.Stderr, "  stateless flow...")
		t0, turns, err := runStatelessFlow(ctx, client, exp, baseArtifact, baseT0)
		if err != nil {
			return Metrics{}, err
		}
		flow := toFlowMetrics(turns)
		metrics.StatelessT0 = &t0
		metrics.StatelessFlow = &flow
	}

	if runGAP {
		fmt.Fprintln(os.Stderr, "  gap flow...")
		t0, turns, err := runGAPFlow(ctx, client, exp)
		if err != nil {
			return Metrics{}, err
		}
		flow := toFlowMetrics(turns)
		gapFlow := GapFlowMetrics{
			FlowMetrics:       flow,
			EnvelopeParseRate: rate(turns, func(t TurnResult) bool { return boolValue(t.EnvelopeParsed) }),
			ApplySuccessRate:  rate(turns, func(t TurnResult) bool { return boolValue(t.ApplySucceeded) }),
		}
		metrics.GAPTurn0 = &t0
		metrics.GAPFlow = &gapFlow
	}

	fillDerived(&metrics)
	return metrics, nil
}

func runBaseFlow(ctx context.Context, client *Client, exp evalset.ExperimentInput) (TurnMetrics, []TurnResult, string, error) {
	outDir := filepath.Join(exp.Paths.ExperimentDir, "outputs", "base")
	if err := os.MkdirAll(outDir, 0o750); err != nil {
		return TurnMetrics{}, nil, "", err
	}
	ext := formatToExt(exp.Format)
	start := time.Now()
	result, err := client.Chat(ctx, []Message{
		{Role: "system", Content: exp.BaseSystem},
		{Role: "user", Content: exp.Turns[0].Prompt},
	}, false)
	if err != nil {
		return TurnMetrics{}, nil, "", err
	}
	artifact := cleanArtifact(result.Text)
	if err := writeText(filepath.Join(outDir, "turn-0"+ext), artifact); err != nil {
		return TurnMetrics{}, nil, "", err
	}
	t0 := turnMetrics(result, start, artifact)

	messages := []Message{
		{Role: "system", Content: exp.BaseSystem},
		{Role: "user", Content: exp.Turns[0].Prompt},
		{Role: "assistant", Content: artifact},
	}
	var turns []TurnResult
	for _, turn := range exp.Turns[1:] {
		messages = append(messages, Message{Role: "user", Content: turn.Prompt})
		start := time.Now()
		result, err := client.Chat(ctx, messages, false)
		if err != nil {
			return t0, turns, artifact, err
		}
		artifact = cleanArtifact(result.Text)
		if err := writeText(filepath.Join(outDir, fmt.Sprintf("turn-%d%s", turn.Turn, ext)), artifact); err != nil {
			return t0, turns, artifact, err
		}
		messages = append(messages, Message{Role: "assistant", Content: artifact})
		turns = append(turns, turnResult(turn, result, start, artifact))
	}
	return t0, turns, artifact, nil
}

func runStatelessFlow(ctx context.Context, client *Client, exp evalset.ExperimentInput, seedArtifact string, seedMetrics *TurnMetrics) (TurnMetrics, []TurnResult, error) {
	outDir := filepath.Join(exp.Paths.ExperimentDir, "outputs", "stateless")
	if err := os.MkdirAll(outDir, 0o750); err != nil {
		return TurnMetrics{}, nil, err
	}
	ext := formatToExt(exp.Format)
	var artifact string
	var t0 TurnMetrics
	if seedArtifact != "" && seedMetrics != nil {
		artifact = seedArtifact
		t0 = *seedMetrics
	} else {
		start := time.Now()
		result, err := client.Chat(ctx, []Message{
			{Role: "system", Content: exp.BaseSystem},
			{Role: "user", Content: exp.Turns[0].Prompt},
		}, false)
		if err != nil {
			return TurnMetrics{}, nil, err
		}
		artifact = cleanArtifact(result.Text)
		t0 = turnMetrics(result, start, artifact)
	}
	if err := writeText(filepath.Join(outDir, "turn-0"+ext), artifact); err != nil {
		return TurnMetrics{}, nil, err
	}

	var turns []TurnResult
	for _, turn := range exp.Turns[1:] {
		user := fmt.Sprintf("## Current Artifact\n\n```\n%s\n```\n\n## Edit Instruction\n\n%s\n\nReturn the complete updated artifact, raw, with no commentary.", artifact, turn.Prompt)
		start := time.Now()
		result, err := client.Chat(ctx, []Message{
			{Role: "system", Content: exp.BaseSystem},
			{Role: "user", Content: user},
		}, false)
		if err != nil {
			return t0, turns, err
		}
		artifact = cleanArtifact(result.Text)
		if err := writeText(filepath.Join(outDir, fmt.Sprintf("turn-%d%s", turn.Turn, ext)), artifact); err != nil {
			return t0, turns, err
		}
		turns = append(turns, turnResult(turn, result, start, artifact))
	}
	return t0, turns, nil
}

func runGAPFlow(ctx context.Context, client *Client, exp evalset.ExperimentInput) (TurnMetrics, []TurnResult, error) {
	outDir := filepath.Join(exp.Paths.ExperimentDir, "outputs", "gap")
	if err := os.MkdirAll(outDir, 0o750); err != nil {
		return TurnMetrics{}, nil, err
	}
	ext := formatToExt(exp.Format)
	t0, artifact, err := runGAPSynthesis(ctx, client, exp)
	if err != nil {
		return TurnMetrics{}, nil, err
	}
	if err := writeText(filepath.Join(outDir, "turn-0"+ext), artifact); err != nil {
		return TurnMetrics{}, nil, err
	}

	var turns []TurnResult
	version := uint64(1)
	for _, turn := range exp.Turns[1:] {
		tr, nextArtifact, nextVersion, envelope, envelopeText := runGAPEditTurn(ctx, client, exp, artifact, version, turn)
		if envelope != nil {
			_ = writeJSON(envelopePath(outDir, turn.Turn, ext), *envelope)
		} else if envelopeText != "" {
			_ = writeText(envelopePath(outDir, turn.Turn, ext), envelopeText)
		}
		artifact = nextArtifact
		version = nextVersion
		if err := writeText(filepath.Join(outDir, fmt.Sprintf("turn-%d%s", turn.Turn, ext)), artifact); err != nil {
			return t0, turns, err
		}
		turns = append(turns, tr)
	}
	return t0, turns, nil
}

func runGAPSynthesis(ctx context.Context, client *Client, exp evalset.ExperimentInput) (TurnMetrics, string, error) {
	start := time.Now()
	messages := []Message{
		{Role: "system", Content: exp.GAPInitSystem},
		{Role: "user", Content: exp.Turns[0].Prompt},
	}
	var aggregate ChatResult
	var artifact string
	var lastErr error
	for attempt := 0; attempt < maxSynthesisAttempts; attempt++ {
		result, err := client.Chat(ctx, messages, false)
		addChatResult(&aggregate, result)
		if err != nil {
			return TurnMetrics{}, "", err
		}
		artifact = cleanArtifact(result.Text)
		if err := validateSynthesisArtifact(artifact, exp.Format); err == nil {
			return turnMetrics(aggregate, start, artifact), artifact, nil
		} else {
			lastErr = err
			messages = append(messages,
				Message{Role: "assistant", Content: result.Text},
				Message{Role: "user", Content: synthesisRepairPrompt(artifact, exp.Format, err)},
			)
		}
	}
	if lastErr != nil {
		return turnMetrics(aggregate, start, artifact), artifact, nil
	}
	return turnMetrics(aggregate, start, artifact), artifact, nil
}

func runGAPEditTurn(ctx context.Context, client *Client, exp evalset.ExperimentInput, artifact string, version uint64, turn evalset.TurnInput) (TurnResult, string, uint64, *gap.Envelope, string) {
	start := time.Now()
	messages := []Message{
		{Role: "system", Content: exp.GAPMaintain},
		{Role: "user", Content: editPrompt(artifact, exp.Format, turn.Prompt)},
	}
	var aggregate ChatResult
	var parsed bool
	var applied bool
	var envelopeName string
	var repairAttempts int
	var lastErr error
	var validationErr error
	var lastEnvelope *gap.Envelope
	var lastEnvelopeText string

	for attempt := 0; attempt < maxEditAttempts; attempt++ {
		result, err := client.Chat(ctx, messages, true)
		addChatResult(&aggregate, result)
		if err != nil {
			lastErr = err
			break
		}
		envelopeText := cleanArtifact(result.Text)
		lastEnvelopeText = envelopeText
		envelope, parseErr := gap.EnvelopeFromJSON([]byte(extractJSONObject(envelopeText)))
		if parseErr != nil {
			lastErr = fmt.Errorf("envelope parse failed: %w", parseErr)
			validationErr = lastErr
			repairAttempts++
			messages = appendRepairMessages(messages, result.Text, artifact, exp.Format, turn.Prompt, lastErr)
			continue
		}
		parsed = true
		envelope = normalizeEnvelope(envelope, exp, version+1)
		envelopeName = string(envelope.Name)
		lastEnvelope = &envelope
		if err := validateEnvelope(artifact, exp.Format, envelope); err != nil {
			lastErr = err
			validationErr = err
			repairAttempts++
			messages = appendRepairMessages(messages, result.Text, artifact, exp.Format, turn.Prompt, err)
			continue
		}
		newArtifact, _, applyErr := gap.Apply(&gap.Artifact{
			ID:      exp.ExperimentID,
			Version: version,
			Format:  exp.Format,
			Body:    artifact,
		}, envelope)
		if applyErr != nil {
			lastErr = fmt.Errorf("apply failed: %w", applyErr)
			validationErr = lastErr
			repairAttempts++
			messages = appendRepairMessages(messages, result.Text, artifact, exp.Format, turn.Prompt, lastErr)
			continue
		}
		tr := turnResult(turn, aggregate, start, newArtifact.Body)
		tr.EnvelopeParsed = &parsed
		applied = true
		tr.ApplySucceeded = &applied
		tr.EnvelopeName = &envelopeName
		tr.RepairAttempts = repairAttempts
		if validationErr != nil {
			value := validationErr.Error()
			tr.ValidationError = &value
		}
		tr.OutputBytes = len(newArtifact.Body)
		return tr, newArtifact.Body, newArtifact.Version, &envelope, lastEnvelopeText
	}

	tr := turnResult(turn, aggregate, start, artifact)
	tr.Failed = true
	if lastErr != nil {
		reason := lastErr.Error()
		if !strings.HasPrefix(reason, "apply failed:") && !strings.HasPrefix(reason, "envelope parse failed:") {
			reason = "validation failed: " + reason
		}
		tr.FailureReason = &reason
	}
	tr.EnvelopeParsed = &parsed
	tr.ApplySucceeded = &applied
	tr.EnvelopeName = &envelopeName
	tr.RepairAttempts = repairAttempts
	if validationErr != nil {
		value := validationErr.Error()
		tr.ValidationError = &value
	}
	return tr, artifact, version, lastEnvelope, lastEnvelopeText
}

func appendRepairMessages(messages []Message, assistantText string, artifact string, format string, instruction string, err error) []Message {
	return append(messages,
		Message{Role: "assistant", Content: assistantText},
		Message{Role: "user", Content: repairPrompt(artifact, format, instruction, err)},
	)
}

func addChatResult(total *ChatResult, result ChatResult) {
	total.Text = result.Text
	total.InputTokens += result.InputTokens
	total.OutputTokens += result.OutputTokens
	total.CachedInputTokens += result.CachedInputTokens
	total.Retried = total.Retried || result.Retried
}

func normalizeEnvelope(envelope gap.Envelope, exp evalset.ExperimentInput, version uint64) gap.Envelope {
	envelope.Protocol = gap.ProtocolVersion
	envelope.ID = exp.ExperimentID
	if envelope.Version < version {
		envelope.Version = version
	}
	format := exp.Format
	envelope.Meta.Format = &format
	return envelope
}

func turnMetrics(result ChatResult, start time.Time, artifact string) TurnMetrics {
	return TurnMetrics{
		InputTokens:       result.InputTokens,
		OutputTokens:      result.OutputTokens,
		CachedInputTokens: result.CachedInputTokens,
		LatencyMS:         uint64(time.Since(start).Milliseconds()),
		ArtifactBytes:     len(artifact),
	}
}

func turnResult(turn evalset.TurnInput, result ChatResult, start time.Time, artifact string) TurnResult {
	retried := result.Retried
	return TurnResult{
		Turn:              turn.Turn,
		Edit:              truncate(turn.Prompt, 80),
		InputTokens:       result.InputTokens,
		OutputTokens:      result.OutputTokens,
		CachedInputTokens: result.CachedInputTokens,
		LatencyMS:         uint64(time.Since(start).Milliseconds()),
		OutputBytes:       len(artifact),
		Retried:           &retried,
		Failed:            false,
	}
}

func fillDerived(metrics *Metrics) {
	if metrics.DefaultFlow != nil && metrics.GAPFlow != nil {
		metrics.Comparison = &Comparison{
			OutputTokenSavingsPct: pct(metrics.DefaultFlow.TotalOutputTokens, metrics.GAPFlow.TotalOutputTokens),
			InputTokenSavingsPct:  pct(metrics.DefaultFlow.TotalInputTokens, metrics.GAPFlow.TotalInputTokens),
			LatencySavingsPct:     pct(metrics.DefaultFlow.TotalLatencyMillis, metrics.GAPFlow.TotalLatencyMillis),
		}
		metrics.Economics = economics(metrics.BaseTurn0, metrics.GAPTurn0, metrics.DefaultFlow, metrics.GAPFlow)
	}
	if metrics.GAPFlow != nil {
		metrics.Reliability = reliability(metrics.GAPFlow.PerTurn)
	}
	if metrics.DefaultFlow != nil && metrics.StatelessFlow != nil && metrics.GAPFlow != nil {
		aIn, aOut := initInclusive(metrics.BaseTurn0, metrics.DefaultFlow)
		bIn, bOut := initInclusive(metrics.StatelessT0, metrics.StatelessFlow)
		cIn, cOut := initInclusive(metrics.GAPTurn0, &metrics.GAPFlow.FlowMetrics)
		metrics.Decomposition = &Decomposition{
			InputSavingsBVsAPct:  pct(aIn, bIn),
			OutputSavingsCVsBPct: pct(bOut, cOut),
			InputSavingsCVsAPct:  pct(aIn, cIn),
			OutputSavingsCVsAPct: pct(aOut, cOut),
		}
	}
	if metrics.DefaultFlow != nil || metrics.GAPFlow != nil {
		metrics.Validity = &Validity{
			GAPRunDegenerate: gapDegenerate(metrics.GAPFlow),
			BaseInputMonotone: metrics.DefaultFlow == nil ||
				inputMonotone(metrics.DefaultFlow.PerTurn),
		}
	}
}

func reliability(turns []TurnResult) *Reliability {
	report := &Reliability{
		EditTurns: len(turns),
		ByReason:  map[string]int{},
	}
	for _, turn := range turns {
		if !turn.Failed {
			continue
		}
		report.MissCount++
		reason := ""
		if turn.FailureReason != nil {
			reason = *turn.FailureReason
			report.ByReason[reason]++
		}
		switch {
		case strings.HasPrefix(reason, "envelope parse failed"):
			report.ParseMissCount++
		case strings.HasPrefix(reason, "validation failed"):
			report.ValidationMissCount++
		case strings.HasPrefix(reason, "invalid envelope"):
			report.InvalidEnvelopeCount++
		case strings.HasPrefix(reason, "apply failed"):
			report.ApplyMissCount++
		case turn.EnvelopeParsed != nil && !*turn.EnvelopeParsed:
			report.RequestFailureCount++
		case boolValue(turn.EnvelopeParsed) && !boolValue(turn.ApplySucceeded):
			report.ApplyMissCount++
		default:
			report.UnknownMissCount++
		}
	}
	if report.MissCount == 0 {
		report.ByReason = nil
	}
	if report.EditTurns > 0 {
		report.MissRate = round1(float64(report.MissCount)/float64(report.EditTurns)*100) / 100
	}
	return report
}

func economics(baseTurn0 *TurnMetrics, gapTurn0 *TurnMetrics, baseFlow *FlowMetrics, gapFlow *GapFlowMetrics) *Economics {
	baseIn, baseOut, gapIn, gapOut := alignedTotals(baseFlow.PerTurn, gapFlow.PerTurn)
	fallbackIn, fallbackOut := gapIn, gapOut
	var missAttemptIn, missAttemptOut, retryIn, retryOut uint64
	for i, gapTurn := range gapFlow.PerTurn {
		if i >= len(baseFlow.PerTurn) || !gapTurn.Failed {
			continue
		}
		baseTurn := baseFlow.PerTurn[i]
		missAttemptIn += gapTurn.InputTokens
		missAttemptOut += gapTurn.OutputTokens
		retryIn += baseTurn.InputTokens
		retryOut += baseTurn.OutputTokens
		fallbackIn += baseTurn.InputTokens
		fallbackOut += baseTurn.OutputTokens
	}

	baseTotal := baseIn + baseOut
	gapTotal := gapIn + gapOut
	fallbackTotal := fallbackIn + fallbackOut
	econ := &Economics{
		FallbackAssumption:           "on each missed GAP edit, run the baseline full-regeneration edit after the failed GAP attempt",
		MeasuredTotalTokenSavingsPct: pct(baseTotal, gapTotal),
		FallbackAdjusted: &FallbackAdjusted{
			InputTokens:               fallbackIn,
			OutputTokens:              fallbackOut,
			TotalTokens:               fallbackTotal,
			MissAttemptInputTokens:    missAttemptIn,
			MissAttemptOutputTokens:   missAttemptOut,
			MissAttemptTotalTokens:    missAttemptIn + missAttemptOut,
			FallbackRetryInputTokens:  retryIn,
			FallbackRetryOutputTokens: retryOut,
			FallbackRetryTotalTokens:  retryIn + retryOut,
			InputTokenSavingsPct:      pct(baseIn, fallbackIn),
			OutputTokenSavingsPct:     pct(baseOut, fallbackOut),
			TotalTokenSavingsPct:      pct(baseTotal, fallbackTotal),
		},
	}
	if baseTurn0 != nil && gapTurn0 != nil {
		baseSession := baseTotal + baseTurn0.InputTokens + baseTurn0.OutputTokens
		gapSession := gapTotal + gapTurn0.InputTokens + gapTurn0.OutputTokens
		fallbackSession := fallbackTotal + gapTurn0.InputTokens + gapTurn0.OutputTokens
		econ.Amortized = &AmortizedEconomics{
			EditTurns:                            len(gapFlow.PerTurn),
			BaseInitInclusiveTokens:              baseSession,
			GAPInitInclusiveTokens:               gapSession,
			FallbackInitInclusiveTokens:          fallbackSession,
			MeasuredInitInclusiveTokenSavingsPct: pct(baseSession, gapSession),
			FallbackInitInclusiveTokenSavingsPct: pct(baseSession, fallbackSession),
		}
	}
	return econ
}

func alignedTotals(baseTurns []TurnResult, gapTurns []TurnResult) (uint64, uint64, uint64, uint64) {
	var baseIn, baseOut, gapIn, gapOut uint64
	limit := len(baseTurns)
	if len(gapTurns) < limit {
		limit = len(gapTurns)
	}
	for i := 0; i < limit; i++ {
		baseIn += baseTurns[i].InputTokens
		baseOut += baseTurns[i].OutputTokens
		gapIn += gapTurns[i].InputTokens
		gapOut += gapTurns[i].OutputTokens
	}
	return baseIn, baseOut, gapIn, gapOut
}

func filterExperiments(experiments []evalset.ExperimentInput, id string, count int) []evalset.ExperimentInput {
	var filtered []evalset.ExperimentInput
	for _, exp := range experiments {
		if id == "" || strings.HasPrefix(exp.ExperimentID, id) {
			filtered = append(filtered, exp)
		}
	}
	if count > 0 && len(filtered) > count {
		filtered = filtered[:count]
	}
	return filtered
}

func cleanArtifact(text string) string {
	s := strings.TrimSpace(thinkRE.ReplaceAllString(text, ""))
	if strings.HasPrefix(s, "```") {
		if newline := strings.IndexByte(s, '\n'); newline >= 0 {
			s = s[newline+1:]
		}
	}
	s = strings.TrimSpace(s)
	if strings.HasSuffix(s, "```") {
		s = strings.TrimSpace(strings.TrimSuffix(s, "```"))
	}
	return s
}

func extractJSONObject(text string) string {
	start := strings.IndexByte(text, '{')
	end := strings.LastIndexByte(text, '}')
	if start >= 0 && end >= start {
		return text[start : end+1]
	}
	return text
}

func envelopePath(outDir string, turn int, ext string) string {
	if ext == ".json" {
		return filepath.Join(outDir, fmt.Sprintf("turn-%d.envelope.json", turn))
	}
	return filepath.Join(outDir, fmt.Sprintf("turn-%d.json", turn))
}

func writeText(path string, value string) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o750); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(value), 0o600)
}

func writeJSON(path string, value any) error {
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return err
	}
	return writeText(path, string(data)+"\n")
}

func rate(turns []TurnResult, pred func(TurnResult) bool) float64 {
	if len(turns) == 0 {
		return 0
	}
	var count int
	for _, turn := range turns {
		if pred(turn) {
			count++
		}
	}
	return float64(count) / float64(len(turns))
}

func boolValue(value *bool) bool {
	return value != nil && *value
}

func pct(base uint64, next uint64) float64 {
	if base == 0 {
		return 0
	}
	return round1((1 - float64(next)/float64(base)) * 100)
}

func initInclusive(t0 *TurnMetrics, flow *FlowMetrics) (uint64, uint64) {
	in, out := flow.TotalInputTokens, flow.TotalOutputTokens
	if t0 != nil {
		in += t0.InputTokens
		out += t0.OutputTokens
	}
	return in, out
}

func gapDegenerate(flow *GapFlowMetrics) bool {
	if flow == nil || len(flow.PerTurn) < 2 {
		return false
	}
	first := flow.PerTurn[0].OutputBytes
	for _, turn := range flow.PerTurn[1:] {
		if turn.OutputBytes != first {
			return false
		}
	}
	return true
}

func inputMonotone(turns []TurnResult) bool {
	for i := 1; i < len(turns); i++ {
		if turns[i].InputTokens < turns[i-1].InputTokens {
			return false
		}
	}
	return true
}

func round1(v float64) float64 {
	if v >= 0 {
		return float64(int(v*10+0.5)) / 10
	}
	return float64(int(v*10-0.5)) / 10
}

func truncate(s string, maxLen int) string {
	s = strings.TrimSpace(s)
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen]
}

func formatToExt(format string) string {
	switch format {
	case "text/html":
		return ".html"
	case "text/x-python":
		return ".py"
	case "application/javascript":
		return ".js"
	case "text/typescript":
		return ".ts"
	case "application/json":
		return ".json"
	case "text/x-yaml":
		return ".yaml"
	case "text/x-toml":
		return ".toml"
	case "text/x-rust":
		return ".rs"
	case "text/x-go":
		return ".go"
	case "text/css":
		return ".css"
	case "text/x-shellscript":
		return ".sh"
	case "text/markdown":
		return ".md"
	case "image/svg+xml":
		return ".svg"
	case "application/xml":
		return ".xml"
	case "text/x-java":
		return ".java"
	case "text/x-ruby":
		return ".rb"
	case "application/sql":
		return ".sql"
	default:
		return ".txt"
	}
}
