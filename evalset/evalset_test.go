package evalset

import (
	"context"
	"encoding/json"
	"testing"

	saigeeval "github.com/urmzd/saige/eval"
)

const experimentsDir = "../assets/evals/experiments"

func TestLoadExperimentsAsSAIGEObservations(t *testing.T) {
	observations, err := LoadObservations(experimentsDir)
	if err != nil {
		t.Fatalf("LoadObservations: %v", err)
	}
	if len(observations) != 96 {
		t.Fatalf("observations = %d, want 96", len(observations))
	}

	first := observations[0]
	if first.ID != "001-html-dashboard-ecommerce" {
		t.Fatalf("first ID = %q", first.ID)
	}
	if first.Turn != 4 {
		t.Fatalf("turn count marker = %d, want 4", first.Turn)
	}

	var input ExperimentInput
	if err := json.Unmarshal(first.Input, &input); err != nil {
		t.Fatalf("input JSON: %v", err)
	}
	if input.Operation != OperationConversation || input.Format != "text/html" {
		t.Fatalf("input metadata = %#v", input)
	}
	if len(input.Turns) != 5 {
		t.Fatalf("turns = %d, want 5", len(input.Turns))
	}
	if len(input.ExpectedSections) == 0 {
		t.Fatal("expected sections should be populated")
	}
	if _, ok := first.Annotations[AnnotationExperimentDir]; !ok {
		t.Fatalf("missing %s annotation", AnnotationExperimentDir)
	}
}

func TestCommittedMetricsRunThroughSAIGEScorers(t *testing.T) {
	observations, err := LoadObservations(experimentsDir)
	if err != nil {
		t.Fatalf("LoadObservations: %v", err)
	}
	observations = FilterWithMetrics(observations)
	if len(observations) < 18 {
		t.Fatalf("metrics observations = %d, want at least 18 committed runs", len(observations))
	}

	if err := saigeeval.Populate(context.Background(), observations, CommittedMetricsSubject()); err != nil {
		t.Fatalf("Populate metrics: %v", err)
	}
	result, err := saigeeval.Run(context.Background(), "gap-committed-metrics", observations, MetricsScorers())
	if err != nil {
		t.Fatalf("Run metrics suite: %v", err)
	}
	for _, metric := range []string{"output_token_savings", "gap_envelope_parse_rate", "gap_apply_success_rate"} {
		if _, ok := result.Aggregate[metric]; !ok {
			t.Fatalf("missing aggregate %q: %#v", metric, result.Aggregate)
		}
	}
	if result.Aggregate["output_token_savings"] <= 0 {
		t.Fatalf("output_token_savings = %f, want positive", result.Aggregate["output_token_savings"])
	}
}
