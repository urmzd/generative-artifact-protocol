package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/urmzd/generative-artifact-protocol/internal/liveeval"
)

func main() {
	var cfg liveeval.Config
	flag.StringVar(&cfg.ExperimentsDir, "experiments-dir", "assets/evals/experiments", "Experiment corpus directory")
	flag.IntVar(&cfg.Count, "count", 0, "Maximum experiments to run, 0 for all")
	flag.StringVar(&cfg.IDFilter, "id", "", "Experiment ID prefix filter")
	flag.StringVar(&cfg.Flow, "flow", "both", "Flow to run: base|stateless|gap|both|abc|all")
	flag.StringVar(&cfg.Model, "model", "gpt-4o-mini", "OpenAI-compatible model name")
	flag.StringVar(&cfg.APIBase, "api-base", envOr("GAP_API_BASE", "https://api.openai.com/v1"), "OpenAI-compatible API base URL")
	flag.StringVar(&cfg.APIKey, "api-key", firstEnv("GAP_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY", "GROQ_API_KEY", "CEREBRAS_API_KEY", "OPENROUTER_API_KEY", "MISTRAL_API_KEY", "GITHUB_TOKEN"), "API key")
	flag.BoolVar(&cfg.Force, "force", false, "Re-run experiments even when metrics.json exists")
	flag.Parse()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if err := liveeval.Run(ctx, cfg); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

func envOr(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}

func firstEnv(keys ...string) string {
	for _, key := range keys {
		if value := os.Getenv(key); value != "" {
			return value
		}
	}
	return ""
}
