package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/urmzd/generative-artifact-protocol/evalset"
)

func main() {
	experimentsDir := "assets/evals/experiments"
	outputPath := "assets/evals/saige/observations.json"
	if len(os.Args) > 1 {
		experimentsDir = os.Args[1]
	}
	if len(os.Args) > 2 {
		outputPath = os.Args[2]
	}

	observations, err := evalset.LoadObservations(experimentsDir)
	if err != nil {
		fatal(err)
	}
	data, err := json.MarshalIndent(observations, "", "  ")
	if err != nil {
		fatal(err)
	}
	if err := os.MkdirAll(filepath.Dir(outputPath), 0o750); err != nil {
		fatal(err)
	}
	if err := os.WriteFile(outputPath, append(data, '\n'), 0o600); err != nil {
		fatal(err)
	}
	fmt.Fprintf(os.Stderr, "wrote %s (%d observations)\n", outputPath, len(observations))
}

func fatal(err error) {
	fmt.Fprintf(os.Stderr, "error: %v\n", err)
	os.Exit(1)
}
