package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

const (
	defaultExperimentsDir = "assets/evals/experiments"
	defaultOutputPath     = "assets/evals/experiments/results.md"

	// Published GPT-4o mini launch rates. Costs in this report are modeled from
	// measured tokens; token counts are the source of truth.
	priceInputPerMillion  = 0.15
	priceOutputPerMillion = 0.60
)

type metricsFile struct {
	ExperimentID string       `json:"experiment_id"`
	Format       string       `json:"format"`
	Model        string       `json:"model"`
	Timestamp    string       `json:"timestamp"`
	BaseTurn0    *turnMetrics `json:"base_turn0,omitempty"`
	GAPTurn0     *turnMetrics `json:"gap_turn0,omitempty"`
	DefaultFlow  *flowMetrics `json:"default_flow,omitempty"`
	GAPFlow      *gapFlow     `json:"gap_flow,omitempty"`
	Reliability  *reliability `json:"reliability,omitempty"`
	Economics    *economics   `json:"economics,omitempty"`
	Validity     *validity    `json:"validity,omitempty"`
	SourcePath   string       `json:"-"`
}

type turnMetrics struct {
	InputTokens   uint64 `json:"input_tokens"`
	OutputTokens  uint64 `json:"output_tokens"`
	ArtifactBytes uint64 `json:"artifact_bytes"`
}

type flowMetrics struct {
	PerTurn           []turnResult `json:"per_turn"`
	TotalInputTokens  uint64       `json:"total_input_tokens"`
	TotalOutputTokens uint64       `json:"total_output_tokens"`
}

type gapFlow struct {
	PerTurn           []turnResult `json:"per_turn"`
	TotalInputTokens  uint64       `json:"total_input_tokens"`
	TotalOutputTokens uint64       `json:"total_output_tokens"`
	EnvelopeParseRate float64      `json:"envelope_parse_rate"`
	ApplySuccessRate  float64      `json:"apply_success_rate"`
}

type turnResult struct {
	Failed         bool   `json:"failed"`
	MissReason     string `json:"miss_reason"`
	RepairAttempts uint64 `json:"repair_attempts"`
}

type reliability struct {
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

type economics struct {
	MeasuredTotalTokenSavingsPct float64             `json:"measured_total_token_savings_pct"`
	FallbackAdjusted             *fallbackAdjusted   `json:"fallback_adjusted,omitempty"`
	Amortized                    *amortizedEconomics `json:"amortized,omitempty"`
}

type fallbackAdjusted struct {
	InputTokens              uint64  `json:"input_tokens"`
	OutputTokens             uint64  `json:"output_tokens"`
	TotalTokens              uint64  `json:"total_tokens"`
	MissAttemptTotalTokens   uint64  `json:"miss_attempt_total_tokens"`
	FallbackRetryTotalTokens uint64  `json:"fallback_retry_total_tokens"`
	InputTokenSavingsPct     float64 `json:"input_token_savings_pct"`
	OutputTokenSavingsPct    float64 `json:"output_token_savings_pct"`
	TotalTokenSavingsPct     float64 `json:"total_token_savings_pct"`
}

type amortizedEconomics struct {
	EditTurns                            int     `json:"edit_turns"`
	BaseInitInclusiveTokens              uint64  `json:"base_init_inclusive_tokens"`
	GAPInitInclusiveTokens               uint64  `json:"gap_init_inclusive_tokens"`
	FallbackInitInclusiveTokens          uint64  `json:"fallback_init_inclusive_tokens"`
	MeasuredInitInclusiveTokenSavingsPct float64 `json:"measured_init_inclusive_token_savings_pct"`
	FallbackInitInclusiveTokenSavingsPct float64 `json:"fallback_init_inclusive_token_savings_pct"`
}

type validity struct {
	GAPRunDegenerate bool `json:"gap_run_degenerate"`
}

type aggregate struct {
	Name                     string
	Count                    int
	Turns                    int
	Misses                   int
	BaseTokens               uint64
	FallbackTokens           uint64
	AmortizedBaseTokens      uint64
	AmortizedFallbackTokens  uint64
	BaseInputTokens          uint64
	BaseOutputTokens         uint64
	FallbackInputTokens      uint64
	FallbackOutputTokens     uint64
	SavedTokens              int64
	BaseCost                 float64
	FallbackCost             float64
	MacroFallbackSavings     float64
	WeightedFallbackSavings  float64
	WeightedAmortizedSavings float64
	MissRate                 float64
	AverageArtifactBytes     float64
	SavedCost                float64
}

func main() {
	experimentsDir := flag.String("experiments-dir", defaultExperimentsDir, "directory containing eval experiments")
	output := flag.String("output", defaultOutputPath, "report output path")
	flag.Parse()

	metrics, err := loadMetrics(*experimentsDir)
	if err != nil {
		fatal(err)
	}
	report := renderReport(metrics)
	if err := os.MkdirAll(filepath.Dir(*output), 0o755); err != nil {
		fatal(err)
	}
	if err := os.WriteFile(*output, report, 0o644); err != nil {
		fatal(err)
	}
}

func loadMetrics(root string) ([]metricsFile, error) {
	var metrics []metricsFile
	err := filepath.WalkDir(root, func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() || filepath.Base(path) != "metrics.json" {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("read %s: %w", path, err)
		}
		var file metricsFile
		if err := json.Unmarshal(data, &file); err != nil {
			return fmt.Errorf("parse %s: %w", path, err)
		}
		file.SourcePath = path
		metrics = append(metrics, file)
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Slice(metrics, func(i, j int) bool {
		return metrics[i].ExperimentID < metrics[j].ExperimentID
	})
	return metrics, nil
}

func renderReport(metrics []metricsFile) []byte {
	var out bytes.Buffer
	models := uniqueStrings(metrics, func(m metricsFile) string { return m.Model })
	first, last := timestampRange(metrics)
	degenerate := count(metrics, func(m metricsFile) bool { return isDegenerate(m) })
	comparable := filter(metrics, comparableEconomics)
	noEconomics := len(metrics) - degenerate - len(comparable)
	reliabilityAgg := aggregateReliability(metrics)
	econAgg := aggregateMetrics("Comparable economics", comparable)

	fmt.Fprintln(&out, "# GAP Experiment Results")
	fmt.Fprintln(&out)
	fmt.Fprintf(&out, "**Models:** %s | **Metrics files:** %d | **Timestamp range:** %s -> %s\n", strings.Join(models, ", "), len(metrics), emptyDash(first), emptyDash(last))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "This report is generated from `metrics.json` files in this working tree. Token counts and reliability are measured. Dollar costs are modeled from measured tokens using GPT-4o mini launch rates: input $0.15/M, output $0.60/M. Degenerate GAP runs and runs without both base and GAP economics are excluded from savings aggregates.")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Validity")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "| Set | Count |")
	fmt.Fprintln(&out, "|---|---:|")
	fmt.Fprintf(&out, "| Metrics files | %d |\n", len(metrics))
	fmt.Fprintf(&out, "| Degenerate GAP runs excluded from economics | %d |\n", degenerate)
	fmt.Fprintf(&out, "| Missing comparable economics excluded from economics | %d |\n", noEconomics)
	fmt.Fprintf(&out, "| Comparable economics set | %d |\n", len(comparable))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Reliability")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "| Scope | Edit turns | Misses | Miss rate | Parse | Validation | Invalid envelope | Apply | Request | Unknown | Repairs |")
	fmt.Fprintln(&out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
	fmt.Fprintf(&out, "| All runs with reliability | %d | %d | %s | %d | %d | %d | %d | %d | %d | %d |\n",
		reliabilityAgg.Turns, reliabilityAgg.Misses, pctString(reliabilityAgg.MissRate),
		reliabilityAgg.ParseMisses, reliabilityAgg.ValidationMisses, reliabilityAgg.InvalidEnvelopeMisses,
		reliabilityAgg.ApplyMisses, reliabilityAgg.RequestMisses, reliabilityAgg.UnknownMisses, reliabilityAgg.Repairs)
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "A miss means the GAP attempt did not produce an applied edit. The production fallback assumption is: pay for the failed GAP attempt, then run the baseline full-regeneration edit.")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Economics")
	fmt.Fprintln(&out)
	writeEconomicsTable(&out, econAgg)
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "Interpretation: GAP is not a blanket cost win. It reliably reduces output tokens, but small artifacts can lose because target inventory, system instructions, and envelope structure add input overhead.")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Where GAP Saves Most")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "### By Artifact Size")
	fmt.Fprintln(&out)
	writeAggregateTable(&out, bucketBySize(comparable))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "### By Edit Count")
	fmt.Fprintln(&out)
	writeAggregateTable(&out, bucketByTurns(comparable))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "### By Format")
	fmt.Fprintln(&out)
	writeAggregateTable(&out, bucketByFormat(comparable))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Largest Absolute Savings")
	fmt.Fprintln(&out)
	writeExperimentTable(&out, topExperiments(comparable, func(m metricsFile) float64 { return savedCost(m) }, true, 15))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Strongest Amortized Percentage Savings")
	fmt.Fprintln(&out)
	writeExperimentTable(&out, topExperiments(comparable, func(m metricsFile) float64 {
		return m.Economics.Amortized.FallbackInitInclusiveTokenSavingsPct
	}, true, 15))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Negative Savings")
	fmt.Fprintln(&out)
	writeExperimentTable(&out, topExperiments(filter(comparable, func(m metricsFile) bool {
		return m.Economics.FallbackAdjusted.TotalTokenSavingsPct < 0
	}), func(m metricsFile) float64 {
		return m.Economics.FallbackAdjusted.TotalTokenSavingsPct
	}, false, 15))
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "## Operating Guidance")
	fmt.Fprintln(&out)
	fmt.Fprintln(&out, "- Prefer GAP for artifacts above roughly 2 KB, repeated records/pages, dashboards, catalogs, feeds, API payloads, and code files with stable section boundaries.")
	fmt.Fprintln(&out, "- Avoid GAP for tiny config files and one-off edits where the marker and supervisor inventory overhead can exceed the full-regeneration baseline.")
	fmt.Fprintln(&out, "- Treat fallback-adjusted and init-inclusive savings as the headline economics. Raw output savings alone overstates value when misses or setup overhead are present.")
	fmt.Fprintln(&out, "- Continue improving miss rate on high-value large HTML/XML/JSON cases first. Those cases dominate absolute token and dollar savings.")
	return out.Bytes()
}

type reliabilityTotals struct {
	Turns                 int
	Misses                int
	MissRate              float64
	ParseMisses           int
	ValidationMisses      int
	InvalidEnvelopeMisses int
	ApplyMisses           int
	RequestMisses         int
	UnknownMisses         int
	Repairs               uint64
}

func aggregateReliability(metrics []metricsFile) reliabilityTotals {
	var totals reliabilityTotals
	for _, m := range metrics {
		if m.Reliability == nil {
			continue
		}
		totals.Turns += m.Reliability.EditTurns
		totals.Misses += m.Reliability.MissCount
		totals.ParseMisses += m.Reliability.ParseMissCount
		totals.ValidationMisses += m.Reliability.ValidationMissCount
		totals.InvalidEnvelopeMisses += m.Reliability.InvalidEnvelopeCount
		totals.ApplyMisses += m.Reliability.ApplyMissCount
		totals.RequestMisses += m.Reliability.RequestFailureCount
		totals.UnknownMisses += m.Reliability.UnknownMissCount
		if m.GAPFlow != nil {
			for _, turn := range m.GAPFlow.PerTurn {
				totals.Repairs += turn.RepairAttempts
			}
		}
	}
	if totals.Turns > 0 {
		totals.MissRate = float64(totals.Misses) / float64(totals.Turns) * 100
	}
	return totals
}

func writeEconomicsTable(out *bytes.Buffer, agg aggregate) {
	fmt.Fprintln(out, "| Metric | Value |")
	fmt.Fprintln(out, "|---|---:|")
	fmt.Fprintf(out, "| Experiments | %d |\n", agg.Count)
	fmt.Fprintf(out, "| Edit turns | %d |\n", agg.Turns)
	fmt.Fprintf(out, "| Misses | %d (%s) |\n", agg.Misses, pctString(agg.MissRate))
	fmt.Fprintf(out, "| Base edit tokens | %s |\n", intString(agg.BaseTokens))
	fmt.Fprintf(out, "| GAP fallback-adjusted edit tokens | %s |\n", intString(agg.FallbackTokens))
	fmt.Fprintf(out, "| Saved edit tokens | %s |\n", intStringSigned(agg.SavedTokens))
	fmt.Fprintf(out, "| Fallback-adjusted token savings | %s |\n", pctString(agg.WeightedFallbackSavings))
	fmt.Fprintf(out, "| Init-inclusive fallback token savings | %s |\n", pctString(agg.WeightedAmortizedSavings))
	fmt.Fprintf(out, "| Base modeled cost | $%.4f |\n", agg.BaseCost)
	fmt.Fprintf(out, "| GAP fallback modeled cost | $%.4f |\n", agg.FallbackCost)
	fmt.Fprintf(out, "| Modeled cost saved | $%.4f |\n", agg.SavedCost)
}

func writeAggregateTable(out *bytes.Buffer, rows []aggregate) {
	fmt.Fprintln(out, "| Segment | Experiments | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |")
	fmt.Fprintln(out, "|---|---:|---:|---:|---:|---:|---:|---:|")
	for _, row := range rows {
		fmt.Fprintf(out, "| %s | %d | %d | %s | %s | %s | %s | $%.4f |\n",
			row.Name, row.Count, row.Turns, pctString(row.MissRate), intStringSigned(row.SavedTokens),
			pctString(row.WeightedFallbackSavings), pctString(row.WeightedAmortizedSavings), row.SavedCost)
	}
}

func writeExperimentTable(out *bytes.Buffer, rows []metricsFile) {
	fmt.Fprintln(out, "| Experiment | Format | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |")
	fmt.Fprintln(out, "|---|---|---:|---:|---:|---:|---:|---:|")
	for _, m := range rows {
		fmt.Fprintf(out, "| `%s` | `%s` | %d | %s | %s | %s | %s | $%.4f |\n",
			m.ExperimentID, m.Format, m.Reliability.EditTurns, ratioPctString(m.Reliability.MissRate),
			intStringSigned(int64(baseTokens(m))-int64(m.Economics.FallbackAdjusted.TotalTokens)),
			pctString(m.Economics.FallbackAdjusted.TotalTokenSavingsPct),
			pctString(m.Economics.Amortized.FallbackInitInclusiveTokenSavingsPct),
			savedCost(m))
	}
}

func bucketBySize(metrics []metricsFile) []aggregate {
	return aggregateBuckets(metrics, func(m metricsFile) string {
		bytes := uint64(0)
		if m.BaseTurn0 != nil {
			bytes = m.BaseTurn0.ArtifactBytes
		}
		switch {
		case bytes < 2_000:
			return "<2KB"
		case bytes < 5_000:
			return "2-5KB"
		case bytes < 10_000:
			return "5-10KB"
		case bytes < 25_000:
			return "10-25KB"
		default:
			return ">=25KB"
		}
	}, []string{"<2KB", "2-5KB", "5-10KB", "10-25KB", ">=25KB"})
}

func bucketByTurns(metrics []metricsFile) []aggregate {
	return aggregateBuckets(metrics, func(m metricsFile) string {
		return fmt.Sprintf("%d edits", m.Reliability.EditTurns)
	}, nil)
}

func bucketByFormat(metrics []metricsFile) []aggregate {
	rows := aggregateBuckets(metrics, func(m metricsFile) string { return m.Format }, nil)
	sort.Slice(rows, func(i, j int) bool {
		return rows[i].SavedCost > rows[j].SavedCost
	})
	return rows
}

func aggregateBuckets(metrics []metricsFile, key func(metricsFile) string, order []string) []aggregate {
	groups := map[string][]metricsFile{}
	for _, m := range metrics {
		groups[key(m)] = append(groups[key(m)], m)
	}
	var rows []aggregate
	if len(order) > 0 {
		for _, name := range order {
			if group := groups[name]; len(group) > 0 {
				rows = append(rows, aggregateMetrics(name, group))
			}
		}
		return rows
	}
	for name, group := range groups {
		rows = append(rows, aggregateMetrics(name, group))
	}
	sort.Slice(rows, func(i, j int) bool {
		if rows[i].Name == rows[j].Name {
			return false
		}
		return rows[i].Name < rows[j].Name
	})
	return rows
}

func aggregateMetrics(name string, metrics []metricsFile) aggregate {
	var agg aggregate
	agg.Name = name
	agg.Count = len(metrics)
	var macro float64
	var bytesTotal uint64
	for _, m := range metrics {
		baseIn, baseOut := baseInputOutput(m)
		fallback := m.Economics.FallbackAdjusted
		amortized := m.Economics.Amortized
		agg.Turns += m.Reliability.EditTurns
		agg.Misses += m.Reliability.MissCount
		agg.BaseInputTokens += baseIn
		agg.BaseOutputTokens += baseOut
		agg.BaseTokens += baseIn + baseOut
		agg.FallbackInputTokens += fallback.InputTokens
		agg.FallbackOutputTokens += fallback.OutputTokens
		agg.FallbackTokens += fallback.TotalTokens
		agg.AmortizedBaseTokens += amortized.BaseInitInclusiveTokens
		agg.AmortizedFallbackTokens += amortized.FallbackInitInclusiveTokens
		macro += fallback.TotalTokenSavingsPct
		if m.BaseTurn0 != nil {
			bytesTotal += m.BaseTurn0.ArtifactBytes
		}
		agg.BaseCost += tokenCost(baseIn, baseOut)
		agg.FallbackCost += tokenCost(fallback.InputTokens, fallback.OutputTokens)
	}
	agg.SavedTokens = int64(agg.BaseTokens) - int64(agg.FallbackTokens)
	agg.SavedCost = agg.BaseCost - agg.FallbackCost
	agg.MacroFallbackSavings = divFloat(macro, float64(agg.Count))
	agg.WeightedFallbackSavings = pctFloat(agg.BaseTokens, agg.FallbackTokens)
	agg.WeightedAmortizedSavings = pctFloat(agg.AmortizedBaseTokens, agg.AmortizedFallbackTokens)
	if agg.Turns > 0 {
		agg.MissRate = float64(agg.Misses) / float64(agg.Turns) * 100
	}
	agg.AverageArtifactBytes = divFloat(float64(bytesTotal), float64(agg.Count))
	return agg
}

func comparableEconomics(m metricsFile) bool {
	return !isDegenerate(m) &&
		m.DefaultFlow != nil &&
		m.GAPFlow != nil &&
		m.Reliability != nil &&
		m.Economics != nil &&
		m.Economics.FallbackAdjusted != nil &&
		m.Economics.Amortized != nil &&
		baseTokens(m) > 0
}

func isDegenerate(m metricsFile) bool {
	return m.Validity != nil && m.Validity.GAPRunDegenerate
}

func topExperiments(metrics []metricsFile, score func(metricsFile) float64, desc bool, limit int) []metricsFile {
	rows := append([]metricsFile(nil), metrics...)
	sort.Slice(rows, func(i, j int) bool {
		if desc {
			return score(rows[i]) > score(rows[j])
		}
		return score(rows[i]) < score(rows[j])
	})
	if len(rows) > limit {
		return rows[:limit]
	}
	return rows
}

func savedCost(m metricsFile) float64 {
	baseIn, baseOut := baseInputOutput(m)
	fb := m.Economics.FallbackAdjusted
	return tokenCost(baseIn, baseOut) - tokenCost(fb.InputTokens, fb.OutputTokens)
}

func baseInputOutput(m metricsFile) (uint64, uint64) {
	if m.DefaultFlow == nil {
		return 0, 0
	}
	return m.DefaultFlow.TotalInputTokens, m.DefaultFlow.TotalOutputTokens
}

func baseTokens(m metricsFile) uint64 {
	in, out := baseInputOutput(m)
	return in + out
}

func tokenCost(input uint64, output uint64) float64 {
	return (float64(input)*priceInputPerMillion + float64(output)*priceOutputPerMillion) / 1_000_000
}

func pctFloat(base uint64, next uint64) float64 {
	if base == 0 {
		return 0
	}
	return (1 - float64(next)/float64(base)) * 100
}

func divFloat(numerator float64, denominator float64) float64 {
	if denominator == 0 {
		return 0
	}
	return numerator / denominator
}

func pctString(value float64) string {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return "-"
	}
	return fmt.Sprintf("%.1f%%", value)
}

func ratioPctString(value float64) string {
	return pctString(value * 100)
}

func intString(value uint64) string {
	return formatInt(int64(value))
}

func intStringSigned(value int64) string {
	return formatInt(value)
}

func formatInt(value int64) string {
	sign := ""
	if value < 0 {
		sign = "-"
		value = -value
	}
	s := fmt.Sprintf("%d", value)
	var parts []string
	for len(s) > 3 {
		parts = append([]string{s[len(s)-3:]}, parts...)
		s = s[:len(s)-3]
	}
	parts = append([]string{s}, parts...)
	return sign + strings.Join(parts, ",")
}

func uniqueStrings(metrics []metricsFile, value func(metricsFile) string) []string {
	seen := map[string]bool{}
	for _, m := range metrics {
		v := value(m)
		if v == "" {
			continue
		}
		seen[v] = true
	}
	var values []string
	for v := range seen {
		values = append(values, v)
	}
	sort.Strings(values)
	if len(values) == 0 {
		return []string{"unknown"}
	}
	return values
}

func timestampRange(metrics []metricsFile) (string, string) {
	var times []string
	for _, m := range metrics {
		if m.Timestamp != "" {
			times = append(times, m.Timestamp)
		}
	}
	sort.Strings(times)
	if len(times) == 0 {
		return "", ""
	}
	return times[0], times[len(times)-1]
}

func emptyDash(value string) string {
	if value == "" {
		return "-"
	}
	return value
}

func filter(metrics []metricsFile, keep func(metricsFile) bool) []metricsFile {
	var out []metricsFile
	for _, m := range metrics {
		if keep(m) {
			out = append(out, m)
		}
	}
	return out
}

func count(metrics []metricsFile, keep func(metricsFile) bool) int {
	total := 0
	for _, m := range metrics {
		if keep(m) {
			total++
		}
	}
	return total
}

func fatal(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}
