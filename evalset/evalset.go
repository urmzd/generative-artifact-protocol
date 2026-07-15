// Package evalset exposes the GAP benchmark corpus as SAIGE eval observations.
package evalset

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"

	saigeeval "github.com/urmzd/saige/eval"
)

const (
	OperationConversation = "conversation"

	AnnotationExperimentDir    = "gap.experiment_dir"
	AnnotationFormat           = "gap.format"
	AnnotationSizeHint         = "gap.size_hint"
	AnnotationExpectedSections = "gap.expected_sections"
	AnnotationMetricsPath      = "gap.metrics_path"
	AnnotationCheckPaths       = "gap.check_paths"
	AnnotationOutputPaths      = "gap.output_paths"
)

type ExperimentInput struct {
	ExperimentID     string      `json:"experiment_id"`
	Operation        string      `json:"operation"`
	Format           string      `json:"format"`
	SizeHint         string      `json:"size_hint"`
	ExpectedSections []string    `json:"expected_sections,omitempty"`
	BaseSystem       string      `json:"base_system"`
	GAPInitSystem    string      `json:"gap_init_system"`
	GAPMaintain      string      `json:"gap_maintain_system"`
	Turns            []TurnInput `json:"turns"`
	Paths            Paths       `json:"paths"`
}

type TurnInput struct {
	Turn       int    `json:"turn"`
	Prompt     string `json:"prompt"`
	PromptPath string `json:"prompt_path"`
}

type Paths struct {
	ExperimentDir  string   `json:"experiment_dir"`
	Metrics        string   `json:"metrics,omitempty"`
	Checks         []string `json:"checks,omitempty"`
	BaseOutputs    []string `json:"base_outputs,omitempty"`
	StatelessFiles []string `json:"stateless_outputs,omitempty"`
	GAPOutputs     []string `json:"gap_outputs,omitempty"`
}

var (
	metaRE     = regexp.MustCompile(`\*\*Format:\*\*\s*([^|]+)\s*\|\s*\*\*Size:\*\*\s*([^|]+)\s*\|\s*\*+Edits:\*\*\s*(\d+)`)
	sectionsRE = regexp.MustCompile(`(?m)^\*\*Expected sections:\*\*\s*(.+)$`)
	turnRE     = regexp.MustCompile(`turn-(\d+)\.md$`)
)

func LoadObservations(experimentsDir string) ([]saigeeval.Observation, error) {
	experiments, err := LoadExperiments(experimentsDir)
	if err != nil {
		return nil, err
	}
	return Observations(experiments)
}

func LoadExperiments(experimentsDir string) ([]ExperimentInput, error) {
	entries, err := os.ReadDir(experimentsDir)
	if err != nil {
		return nil, fmt.Errorf("read experiments dir: %w", err)
	}

	var experiments []ExperimentInput
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		dir := filepath.Join(experimentsDir, entry.Name())
		exp, err := loadExperiment(dir)
		if err != nil {
			return nil, err
		}
		experiments = append(experiments, exp)
	}
	sort.Slice(experiments, func(i, j int) bool {
		return experiments[i].ExperimentID < experiments[j].ExperimentID
	})
	return experiments, nil
}

func Observations(experiments []ExperimentInput) ([]saigeeval.Observation, error) {
	observations := make([]saigeeval.Observation, 0, len(experiments))
	for _, exp := range experiments {
		input, err := rawJSON(exp)
		if err != nil {
			return nil, fmt.Errorf("%s: marshal input: %w", exp.ExperimentID, err)
		}
		annotations, err := annotationsFor(exp)
		if err != nil {
			return nil, fmt.Errorf("%s: marshal annotations: %w", exp.ExperimentID, err)
		}
		observations = append(observations, saigeeval.Observation{
			ID:          exp.ExperimentID,
			Turn:        max(0, len(exp.Turns)-1),
			Input:       input,
			Annotations: annotations,
		})
	}
	return observations, nil
}

func FilterWithMetrics(observations []saigeeval.Observation) []saigeeval.Observation {
	filtered := make([]saigeeval.Observation, 0, len(observations))
	for _, obs := range observations {
		if raw := obs.Annotations[AnnotationMetricsPath]; len(raw) > 0 && string(raw) != `""` {
			filtered = append(filtered, obs)
		}
	}
	return filtered
}

func loadExperiment(dir string) (ExperimentInput, error) {
	readme, err := readText(filepath.Join(dir, "README.md"))
	if err != nil {
		return ExperimentInput{}, err
	}
	format, sizeHint, edits, err := parseReadme(readme)
	if err != nil {
		return ExperimentInput{}, fmt.Errorf("%s: %w", dir, err)
	}

	turns, err := loadTurns(dir)
	if err != nil {
		return ExperimentInput{}, err
	}
	if len(turns) != edits+1 {
		return ExperimentInput{}, fmt.Errorf("%s: README declares %d edits but found %d turn files", dir, edits, len(turns))
	}
	baseSystem, err := readText(filepath.Join(dir, "inputs", "base", "system.md"))
	if err != nil {
		return ExperimentInput{}, err
	}
	gapInit, err := readText(filepath.Join(dir, "inputs", "gap", "init-system.md"))
	if err != nil {
		return ExperimentInput{}, err
	}
	gapMaintain, err := readText(filepath.Join(dir, "inputs", "gap", "maintain-system.md"))
	if err != nil {
		return ExperimentInput{}, err
	}

	return ExperimentInput{
		ExperimentID:     filepath.Base(dir),
		Operation:        OperationConversation,
		Format:           format,
		SizeHint:         sizeHint,
		ExpectedSections: parseSections(readme),
		BaseSystem:       baseSystem,
		GAPInitSystem:    gapInit,
		GAPMaintain:      gapMaintain,
		Turns:            turns,
		Paths: Paths{
			ExperimentDir:  slash(dir),
			Metrics:        existing(filepath.Join(dir, "metrics.json")),
			Checks:         globSlash(filepath.Join(dir, "checks", "turn-*.json")),
			BaseOutputs:    globSlash(filepath.Join(dir, "outputs", "base", "turn-*")),
			StatelessFiles: globSlash(filepath.Join(dir, "outputs", "stateless", "turn-*")),
			GAPOutputs:     globSlash(filepath.Join(dir, "outputs", "gap", "turn-*")),
		},
	}, nil
}

func loadTurns(dir string) ([]TurnInput, error) {
	matches, err := filepath.Glob(filepath.Join(dir, "inputs", "base", "turn-*.md"))
	if err != nil {
		return nil, err
	}
	sort.Slice(matches, func(i, j int) bool {
		return turnNumber(matches[i]) < turnNumber(matches[j])
	})

	turns := make([]TurnInput, 0, len(matches))
	for _, path := range matches {
		turn := turnNumber(path)
		if turn < 0 {
			continue
		}
		prompt, err := readText(path)
		if err != nil {
			return nil, err
		}
		turns = append(turns, TurnInput{
			Turn:       turn,
			Prompt:     prompt,
			PromptPath: slash(path),
		})
	}
	if len(turns) == 0 || turns[0].Turn != 0 {
		return nil, fmt.Errorf("%s: missing turn-0.md", dir)
	}
	return turns, nil
}

func annotationsFor(exp ExperimentInput) (map[string]json.RawMessage, error) {
	outputs := map[string][]string{
		"base":      exp.Paths.BaseOutputs,
		"stateless": exp.Paths.StatelessFiles,
		"gap":       exp.Paths.GAPOutputs,
	}
	values := map[string]any{
		AnnotationExperimentDir:    exp.Paths.ExperimentDir,
		AnnotationFormat:           exp.Format,
		AnnotationSizeHint:         exp.SizeHint,
		AnnotationExpectedSections: exp.ExpectedSections,
		AnnotationCheckPaths:       exp.Paths.Checks,
		AnnotationOutputPaths:      outputs,
	}
	if exp.Paths.Metrics != "" {
		values[AnnotationMetricsPath] = exp.Paths.Metrics
	}

	annotations := make(map[string]json.RawMessage, len(values))
	for key, value := range values {
		raw, err := rawJSON(value)
		if err != nil {
			return nil, err
		}
		annotations[key] = raw
	}
	return annotations, nil
}

func parseReadme(readme string) (format, sizeHint string, edits int, err error) {
	match := metaRE.FindStringSubmatch(readme)
	if match == nil {
		return "", "", 0, fmt.Errorf("README missing format/size/edit metadata")
	}
	edits, err = strconv.Atoi(strings.TrimSpace(match[3]))
	if err != nil {
		return "", "", 0, fmt.Errorf("invalid edits count: %w", err)
	}
	return strings.TrimSpace(match[1]), strings.TrimSpace(match[2]), edits, nil
}

func parseSections(readme string) []string {
	match := sectionsRE.FindStringSubmatch(readme)
	if match == nil {
		return nil
	}
	parts := strings.Split(match[1], ",")
	sections := make([]string, 0, len(parts))
	for _, part := range parts {
		section := strings.TrimSpace(part)
		if section != "" {
			sections = append(sections, section)
		}
	}
	return sections
}

func turnNumber(path string) int {
	match := turnRE.FindStringSubmatch(filepath.Base(path))
	if match == nil {
		return -1
	}
	turn, err := strconv.Atoi(match[1])
	if err != nil {
		return -1
	}
	return turn
}

func readText(path string) (string, error) {
	data, err := os.ReadFile(filepath.Clean(path))
	if err != nil {
		return "", fmt.Errorf("read %s: %w", path, err)
	}
	return string(data), nil
}

func existing(path string) string {
	if _, err := os.Stat(path); err == nil {
		return slash(path)
	}
	return ""
}

func globSlash(pattern string) []string {
	matches, _ := filepath.Glob(pattern)
	sort.Strings(matches)
	out := make([]string, 0, len(matches))
	for _, match := range matches {
		out = append(out, slash(match))
	}
	return out
}

func slash(path string) string {
	return filepath.ToSlash(filepath.Clean(path))
}

func rawJSON(v any) (json.RawMessage, error) {
	data, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	return json.RawMessage(data), nil
}
