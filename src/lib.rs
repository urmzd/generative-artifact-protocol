pub mod aap;
pub mod apply;
pub mod markers;
pub mod store;
pub mod telemetry;

use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

use tokio::fs;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{info, info_span, Instrument};

use crate::telemetry::Metrics;

/// Watches a file for changes and broadcasts the content on each modification.
pub fn spawn_file_watcher(
    tx: broadcast::Sender<String>,
    file_path: String,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(
        async move {
            let mut last_modified = None;
            let mut tick = tokio::time::interval(interval);
            let metrics = metrics_if_init();
            loop {
                let poll_start = Instant::now();
                tick.tick().await;
                if let Ok(meta) = fs::metadata(&file_path).await {
                    let modified = meta.modified().ok();
                    if modified != last_modified {
                        last_modified = modified;
                        if let Ok(content) = fs::read_to_string(&file_path).await {
                            let file_size = content.len();
                            info!(file_size, path = %file_path, "file change detected");
                            if let Some(m) = metrics {
                                m.watcher_changes_detected.fetch_add(1, Relaxed);
                            }
                            let _ = tx.send(content);
                        }
                    }
                }
                if let Some(m) = metrics {
                    m.record_poll(poll_start.elapsed().as_secs_f64() * 1000.0);
                }
            }
        }
        .instrument(info_span!("file_watcher")),
    )
}

/// Returns `Some(&Metrics)` if telemetry has been initialised, `None` otherwise.
/// This keeps benchmarks (which never call `init()`) working without overhead.
fn metrics_if_init() -> Option<&'static Metrics> {
    std::panic::catch_unwind(Metrics::get).ok()
}
