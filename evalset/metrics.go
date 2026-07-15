package evalset

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	saigeeval "github.com/urmzd/saige/eval"
)

type Metrics struct {
	Comparison *struct {
		InputTokenSavingsPct  float64 `json:"input_token_savings_pct"`
		OutputTokenSavingsPct float64 `json:"output_token_savings_pct"`
		LatencySavingsPct     float64 `json:"latency_savings_pct"`
	} `json:"comparison,omitempty"`
	GAPFlow *struct {
		EnvelopeParseRate float64 `json:"envelope_parse_rate"`
		ApplySuccessRate  float64 `json:"apply_success_rate"`
	} `json:"gap_flow,omitempty"`
	Quality *struct {
		MeanSequenceSimilarity float64 `json:"mean_sequence_similarity"`
		MeanTokenF1            float64 `json:"mean_token_f1"`
		MeanRougeL             float64 `json:"mean_rouge_l"`
	} `json:"quality,omitempty"`
	Correctness *struct {
		PassRate     float64 `json:"pass_rate"`
		BasePassRate float64 `json:"base_pass_rate"`
	} `json:"correctness,omitempty"`
}

func CommittedMetricsSubject() saigeeval.Subject {
	return func(_ context.Context, obs *saigeeval.Observation) error {
		path, ok, err := metricsPath(*obs)
		if err != nil {
			return err
		}
		if !ok || path == "" {
			obs.Output = json.RawMessage(`null`)
			return nil
		}
		data, err := os.ReadFile(filepath.Clean(path))
		if err != nil {
			return fmt.Errorf("read committed metrics: %w", err)
		}
		obs.Output = append(obs.Output[:0], data...)
		return nil
	}
}

func metricsPath(obs saigeeval.Observation) (string, bool, error) {
	path, ok, err := stringAnnotation(obs, AnnotationMetricsPath)
	if err != nil || ok {
		return path, ok, err
	}
	dir, ok, err := stringAnnotation(obs, AnnotationExperimentDir)
	if err != nil || !ok || dir == "" {
		return "", false, err
	}
	return filepath.Join(dir, "metrics.json"), true, nil
}

func MetricsScorers() []saigeeval.Scorer {
	return []saigeeval.Scorer{
		metricScorer("output_token_savings", func(m Metrics) (float64, bool) {
			if m.Comparison == nil {
				return 0, false
			}
			return m.Comparison.OutputTokenSavingsPct / 100, true
		}),
		metricScorer("input_token_savings", func(m Metrics) (float64, bool) {
			if m.Comparison == nil {
				return 0, false
			}
			return m.Comparison.InputTokenSavingsPct / 100, true
		}),
		metricScorer("latency_savings", func(m Metrics) (float64, bool) {
			if m.Comparison == nil {
				return 0, false
			}
			return m.Comparison.LatencySavingsPct / 100, true
		}),
		metricScorer("gap_envelope_parse_rate", func(m Metrics) (float64, bool) {
			if m.GAPFlow == nil {
				return 0, false
			}
			return m.GAPFlow.EnvelopeParseRate, true
		}),
		metricScorer("gap_apply_success_rate", func(m Metrics) (float64, bool) {
			if m.GAPFlow == nil {
				return 0, false
			}
			return m.GAPFlow.ApplySuccessRate, true
		}),
		metricScorer("sequence_similarity", func(m Metrics) (float64, bool) {
			if m.Quality == nil {
				return 0, false
			}
			return m.Quality.MeanSequenceSimilarity, true
		}),
		metricScorer("token_f1", func(m Metrics) (float64, bool) {
			if m.Quality == nil {
				return 0, false
			}
			return m.Quality.MeanTokenF1, true
		}),
		metricScorer("rouge_l", func(m Metrics) (float64, bool) {
			if m.Quality == nil {
				return 0, false
			}
			return m.Quality.MeanRougeL, true
		}),
		metricScorer("correctness_pass_rate", func(m Metrics) (float64, bool) {
			if m.Correctness == nil {
				return 0, false
			}
			return m.Correctness.PassRate, true
		}),
		metricScorer("base_correctness_pass_rate", func(m Metrics) (float64, bool) {
			if m.Correctness == nil {
				return 0, false
			}
			return m.Correctness.BasePassRate, true
		}),
	}
}

func metricScorer(name string, value func(Metrics) (float64, bool)) saigeeval.Scorer {
	return saigeeval.NewScorerFunc(name, func(_ context.Context, obs saigeeval.Observation) (saigeeval.Score, error) {
		if len(obs.Output) == 0 || string(obs.Output) == "null" {
			return saigeeval.Score{}, nil
		}
		var metrics Metrics
		if err := json.Unmarshal(obs.Output, &metrics); err != nil {
			return saigeeval.Score{}, fmt.Errorf("parse metrics output: %w", err)
		}
		score, ok := value(metrics)
		if !ok {
			return saigeeval.Score{}, nil
		}
		return saigeeval.Score{Name: name, Value: score}, nil
	})
}

func stringAnnotation(obs saigeeval.Observation, key string) (string, bool, error) {
	raw, ok := obs.Annotations[key]
	if !ok || len(raw) == 0 {
		return "", false, nil
	}
	var value string
	if err := json.Unmarshal(raw, &value); err != nil {
		return "", false, fmt.Errorf("annotation %s: %w", key, err)
	}
	return value, true, nil
}
