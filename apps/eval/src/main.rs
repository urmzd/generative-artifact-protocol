mod client;
mod experiment;
mod report;
mod runner;
mod scorer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gap-eval", about = "GAP evaluation CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run conversation benchmark experiments (base vs GAP flows)
    Run {
        /// Experiments directory
        #[arg(long, default_value = "assets/evals/experiments")]
        experiments_dir: PathBuf,

        /// Max experiments to run (0 = all)
        #[arg(long, default_value_t = 0)]
        count: usize,

        /// Run a single experiment by ID prefix
        #[arg(long)]
        id: Option<String>,

        /// Which flows to run: base, gap, both
        #[arg(long, default_value = "both")]
        flow: String,

        /// Model name (e.g. gpt-4o-mini, gemini-2.5-flash)
        #[arg(long)]
        model: Option<String>,

        /// OpenAI-compatible API base URL. Works with Gemini, Groq, Cerebras,
        /// OpenRouter, Mistral, GitHub Models, etc. — see README "Running evals
        /// on a free tier" for endpoints.
        #[arg(long, env = "GAP_API_BASE", default_value = "https://api.openai.com/v1")]
        api_base: String,

        /// API key. Falls back to OPENAI_API_KEY, then GEMINI_API_KEY /
        /// GOOGLE_API_KEY / GROQ_API_KEY / CEREBRAS_API_KEY / OPENROUTER_API_KEY
        /// / MISTRAL_API_KEY / GITHUB_TOKEN in that order.
        #[arg(long, env = "GAP_API_KEY")]
        api_key: Option<String>,

        /// Skip quality scoring after runs
        #[arg(long)]
        skip_eval: bool,
    },

    /// Generate a report from experiment metrics
    Report {
        /// Experiments directory
        #[arg(long, default_value = "assets/evals/experiments")]
        experiments_dir: PathBuf,

        /// Output format: human, json
        #[arg(long, default_value = "human")]
        format: String,

        /// Output file (stdout if omitted)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Retroactive quality scoring on completed experiments
    Score {
        /// Experiments directory
        #[arg(long, default_value = "assets/evals/experiments")]
        experiments_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            experiments_dir,
            count,
            id,
            flow,
            model,
            api_base,
            api_key,
            skip_eval,
        } => {
            let api_key = api_key
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .or_else(|| std::env::var("GEMINI_API_KEY").ok())
                .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                .or_else(|| std::env::var("GROQ_API_KEY").ok())
                .or_else(|| std::env::var("CEREBRAS_API_KEY").ok())
                .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
                .or_else(|| std::env::var("MISTRAL_API_KEY").ok())
                .or_else(|| std::env::var("GITHUB_TOKEN").ok())
                .unwrap_or_default();

            let model = model.unwrap_or_else(|| "gpt-4o-mini".to_string());

            let config = experiment::RunConfig {
                experiments_dir,
                count,
                id_filter: id,
                flow,
                model,
                api_base,
                api_key,
                skip_eval,
            };

            experiment::run_all(&config).await?;
        }

        Command::Report {
            experiments_dir,
            format,
            output,
        } => {
            report::generate(&experiments_dir, &format, output.as_deref())?;
        }

        Command::Score { experiments_dir } => {
            scorer::score_all(&experiments_dir)?;
        }
    }

    Ok(())
}
