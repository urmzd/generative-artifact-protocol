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
	start := time.Now()
	result, err := client.Chat(ctx, []Message{
		{Role: "system", Content: exp.GAPInitSystem},
		{Role: "user", Content: exp.Turns[0].Prompt},
	}, false)
	if err != nil {
		return TurnMetrics{}, nil, err
	}
	artifact := cleanArtifact(result.Text)
	if err := writeText(filepath.Join(outDir, "turn-0"+ext), artifact); err != nil {
		return TurnMetrics{}, nil, err
	}
	t0 := turnMetrics(result, start, artifact)

	var turns []TurnResult
	version := uint64(1)
	for _, turn := range exp.Turns[1:] {
		user := fmt.Sprintf("## Current Artifact\n\n```\n%s\n```\n\n## Edit Instruction\n\n%s\n\n## Required Output\n\nReturn exactly one JSON GAP edit envelope with `name` set to `edit` and `content` set to an array of edit operations. Each operation must have `op`, `target`, and `content`. Put replacement text in the operation `content` field.", artifact, turn.Prompt)
		start := time.Now()
		result, err := client.Chat(ctx, []Message{
			{Role: "system", Content: exp.GAPMaintain},
			{Role: "user", Content: user},
		}, true)
		tr := turnResult(turn, result, start, artifact)
		parsed := false
		applied := false
		envelopeName := ""
		if err != nil {
			reason := err.Error()
			tr.Failed = true
			tr.FailureReason = &reason
		} else {
			envelopeText := cleanArtifact(result.Text)
			envelope, parseErr := gap.EnvelopeFromJSON([]byte(extractJSONObject(envelopeText)))
			if parseErr != nil {
				reason := "envelope parse failed: " + parseErr.Error()
				tr.Failed = true
				tr.FailureReason = &reason
				_ = writeText(envelopePath(outDir, turn.Turn, ext), envelopeText)
			} else {
				parsed = true
				envelope = normalizeEnvelope(envelope, exp, version+1)
				envelopeName = string(envelope.Name)
				_ = writeJSON(envelopePath(outDir, turn.Turn, ext), envelope)
				if envelope.Name != gap.NameEdit || len(envelope.Content) == 0 {
					reason := fmt.Sprintf("invalid envelope: name=%q content_items=%d", envelope.Name, len(envelope.Content))
					tr.Failed = true
					tr.FailureReason = &reason
					tr.EnvelopeParsed = &parsed
					tr.ApplySucceeded = &applied
					tr.EnvelopeName = &envelopeName
					turns = append(turns, tr)
					if err := writeText(filepath.Join(outDir, fmt.Sprintf("turn-%d%s", turn.Turn, ext)), artifact); err != nil {
						return t0, turns, err
					}
					continue
				}
				newArtifact, _, applyErr := gap.Apply(&gap.Artifact{
					ID:      exp.ExperimentID,
					Version: version,
					Format:  exp.Format,
					Body:    artifact,
				}, envelope)
				if applyErr != nil {
					reason := "apply failed: " + applyErr.Error()
					tr.Failed = true
					tr.FailureReason = &reason
				} else {
					artifact = newArtifact.Body
					version = newArtifact.Version
					applied = true
					tr.OutputBytes = len(artifact)
				}
			}
		}
		tr.EnvelopeParsed = &parsed
		tr.ApplySucceeded = &applied
		tr.EnvelopeName = &envelopeName
		if err := writeText(filepath.Join(outDir, fmt.Sprintf("turn-%d%s", turn.Turn, ext)), artifact); err != nil {
			return t0, turns, err
		}
		turns = append(turns, tr)
	}
	return t0, turns, nil
}

func normalizeEnvelope(envelope gap.Envelope, exp evalset.ExperimentInput, version uint64) gap.Envelope {
	envelope.Protocol = gap.ProtocolVersion
	envelope.ID = exp.ExperimentID
	if envelope.Version < version {
		envelope.Version = version
	}
	if envelope.Meta.Format == nil || *envelope.Meta.Format == "" {
		format := exp.Format
		envelope.Meta.Format = &format
	}
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
