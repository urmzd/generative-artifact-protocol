use std::time::Duration;

use aap::aap::Envelope;
use aap::store::ArtifactStore;
use aap::telemetry;
use aap::spawn_file_watcher;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let guard = telemetry::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <input> [--watch] [--output <file>]",
            args[0]
        );
        std::process::exit(1);
    }

    let input_path = args[1].clone();

    let mut output_path: Option<String> = None;
    let mut watch_mode = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--output" if i + 1 < args.len() => {
                output_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--watch" => {
                watch_mode = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    if watch_mode {
        info!(input = %input_path, "watching");

        let (tx, mut rx) = broadcast::channel::<String>(16);
        spawn_file_watcher(tx, input_path.clone(), Duration::from_millis(100));

        let metrics = telemetry::Metrics::get();

        let forward = tokio::spawn(async move {
            let mut store = ArtifactStore::new(10);
            loop {
                match rx.recv().await {
                    Ok(content) => {
                        let resolved = resolve_content(&content, &mut store);
                        write_output(&resolved, output_path.as_deref()).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(lagged = n, "watcher lagged");
                        metrics
                            .broadcast_lag_count
                            .fetch_add(n, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        tokio::signal::ctrl_c().await?;
        info!("shutting down");
        forward.abort();
    } else {
        // One-shot mode: read, resolve, output
        let content = tokio::fs::read_to_string(&input_path).await?;
        let mut store = ArtifactStore::new(10);
        let resolved = resolve_content(&content, &mut store);
        write_output(&resolved, output_path.as_deref()).await;
    }

    guard.shutdown();
    Ok(())
}

fn resolve_content(content: &str, store: &mut ArtifactStore) -> String {
    if Envelope::is_envelope(content) {
        match Envelope::from_json(content) {
            Ok(envelope) => {
                info!(
                    id = %envelope.id,
                    version = envelope.version,
                    name = ?envelope.name,
                    "envelope received"
                );
                match store.apply(&envelope) {
                    Ok(resolved) => return resolved,
                    Err(e) => {
                        tracing::error!("envelope apply failed: {e:#}");
                    }
                }
            }
            Err(e) => {
                tracing::error!("envelope parse failed: {e}");
            }
        }
    }
    // Not an envelope or parse failed — return raw content
    content.to_string()
}

async fn write_output(content: &str, output_path: Option<&str>) {
    if let Some(path) = output_path {
        if let Err(e) = tokio::fs::write(path, content).await {
            tracing::error!("failed to write output: {e}");
        }
    } else {
        print!("{content}");
    }
}
