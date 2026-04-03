//! Versioned artifact store — stateful layer over the stateless apply engine.
//!
//! Manages version chains, history, and produces control-plane envelopes
//! (handle/result). The apply engine itself is a pure function; the store
//! provides persistence and orchestration concerns on top.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::aap::{
    ArtifactState, Envelope, FullContentItem, HandleContentItem, Name,
    Operation, ResultContentItem, ResultStatus, PROTOCOL_VERSION,
};
use crate::apply;

/// Entry in the version history.
#[derive(Debug, Clone)]
struct VersionEntry {
    version: u64,
    envelope: Envelope,
}

/// In-memory versioned artifact store.
///
/// Maintains a history of resolved artifact envelopes for each artifact ID,
/// enabling version chain integrity checks and rollback.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    /// artifact_id -> version history (oldest first)
    history: HashMap<String, Vec<VersionEntry>>,
    /// Maximum versions to keep per artifact
    max_history: usize,
}

impl ArtifactStore {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: HashMap::new(),
            max_history,
        }
    }

    /// Get the current artifact envelope.
    pub fn get(&self, id: &str) -> Option<&Envelope> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| &e.envelope)
    }

    /// Get the current version number for an artifact.
    pub fn current_version(&self, id: &str) -> Option<u64> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| e.version)
    }

    /// Apply an envelope, resolving its content and storing the result.
    ///
    /// Enforces version chain integrity: for non-full names, the stored
    /// version must equal `envelope.version - 1` (implicit base_version).
    ///
    /// Returns an output envelope:
    /// - `name:"handle"` for creation operations (`full`, `manifest`)
    /// - `name:"result"` for edit operations (`diff`, `section`, `template`, `composite`)
    pub fn apply(&mut self, operation: &Envelope) -> Result<Envelope> {
        // Version chain check for incremental operations
        if operation.name != Name::Full {
            if let Some(current) = self.current_version(&operation.id) {
                if current != operation.version - 1 {
                    bail!(
                        "version conflict: stored version={current}, envelope version={}, expected stored=={}",
                        operation.version,
                        operation.version - 1
                    );
                }
            } else {
                bail!(
                    "no base content for artifact '{}' — full mode required first",
                    operation.id
                );
            }
        }

        let artifact = self.get(&operation.id);
        let resolved = apply::apply(artifact, operation)?;

        // Extract body for checksum
        let body: FullContentItem = serde_json::from_value(
            resolved.content.first().context("empty resolved content")?.clone(),
        )
        .context("failed to parse resolved content")?;

        let checksum = {
            let mut hasher = Sha256::new();
            hasher.update(body.body.as_bytes());
            format!("sha256:{:x}", hasher.finalize())
        };

        let entries = self.history.entry(operation.id.clone()).or_default();
        entries.push(VersionEntry {
            version: operation.version,
            envelope: resolved,
        });

        // Trim old versions
        while entries.len() > self.max_history {
            entries.remove(0);
        }

        // Build output control-plane envelope
        let output_operation = Operation {
            direction: "output".to_string(),
            format: operation.operation.format.clone(),
            encoding: None,
            content_encoding: None,
            section_id: None,
            token_budget: None,
            tokens_used: operation.operation.tokens_used,
            checksum: Some(checksum.clone()),
            created_at: None,
            updated_at: None,
            state: operation.operation.state.clone(),
            state_changed_at: None,
        };

        let output = match operation.name {
            Name::Full | Name::Manifest => {
                let sections = body.sections.unwrap_or_default();

                let handle = HandleContentItem {
                    sections,
                    token_count: None,
                    state: operation
                        .operation
                        .state
                        .clone()
                        .or(Some(ArtifactState::Draft)),
                };

                Envelope {
                    protocol: PROTOCOL_VERSION.to_string(),
                    id: operation.id.clone(),
                    version: operation.version,
                    name: Name::Handle,
                    operation: output_operation,
                    content: vec![serde_json::to_value(handle)
                        .context("failed to serialize handle content")?],
                }
            }
            _ => {
                let result = ResultContentItem {
                    status: ResultStatus::Applied,
                    mode_used: operation.name.clone(),
                    changes: Vec::new(),
                    tokens_used: operation.operation.tokens_used,
                    rejection_reason: None,
                    conflict_detail: None,
                    checksum: Some(checksum),
                };

                Envelope {
                    protocol: PROTOCOL_VERSION.to_string(),
                    id: operation.id.clone(),
                    version: operation.version,
                    name: Name::Result,
                    operation: output_operation,
                    content: vec![serde_json::to_value(result)
                        .context("failed to serialize result content")?],
                }
            }
        };

        Ok(output)
    }

    /// Roll back to a previous version. Returns the restored artifact envelope.
    pub fn rollback(&mut self, id: &str, target_version: u64) -> Result<Envelope> {
        let entries = self
            .history
            .get_mut(id)
            .context("artifact not found")?;

        let idx = entries
            .iter()
            .position(|e| e.version == target_version)
            .with_context(|| format!("version {target_version} not in history"))?;

        let envelope = entries[idx].envelope.clone();
        let new_version = entries.last().map(|e| e.version).unwrap_or(0) + 1;

        entries.push(VersionEntry {
            version: new_version,
            envelope: envelope.clone(),
        });

        while entries.len() > self.max_history {
            entries.remove(0);
        }

        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aap::*;

    fn make_operation(format: &str) -> Operation {
        Operation {
            direction: "output".to_string(),
            format: Some(format.to_string()),
            encoding: None,
            content_encoding: None,
            section_id: None,
            token_budget: None,
            tokens_used: None,
            checksum: None,
            created_at: None,
            updated_at: None,
            state: None,
            state_changed_at: None,
        }
    }

    fn full_envelope(id: &str, version: u64, body: &str) -> Envelope {
        Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: id.to_string(),
            version,
            name: Name::Full,
            operation: make_operation("text/html"),
            content: vec![serde_json::json!({ "body": body })],
        }
    }

    fn full_envelope_with_sections(
        id: &str,
        version: u64,
        body: &str,
        sections: Vec<SectionDef>,
    ) -> Envelope {
        Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: id.to_string(),
            version,
            name: Name::Full,
            operation: make_operation("text/html"),
            content: vec![serde_json::json!({ "body": body, "sections": sections })],
        }
    }

    fn diff_envelope(id: &str, version: u64, ops: Vec<DiffOp>) -> Envelope {
        let content: Vec<serde_json::Value> = ops
            .iter()
            .map(|op| serde_json::to_value(op).unwrap())
            .collect();
        Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: id.to_string(),
            version,
            name: Name::Diff,
            operation: make_operation("text/html"),
            content,
        }
    }

    /// Helper to extract body from a stored artifact.
    fn get_body(store: &ArtifactStore, id: &str) -> String {
        let env = store.get(id).unwrap();
        let item: FullContentItem =
            serde_json::from_value(env.content[0].clone()).unwrap();
        item.body
    }

    #[test]
    fn test_full_then_diff() {
        let mut store = ArtifactStore::new(10);

        let env1 = full_envelope("test", 1, "<div>hello world</div>");
        let output = store.apply(&env1).unwrap();
        assert_eq!(output.name, Name::Handle);
        assert_eq!(get_body(&store, "test"), "<div>hello world</div>");

        let env2 = diff_envelope(
            "test",
            2,
            vec![DiffOp {
                op: OpType::Replace,
                target: Target {
                    search: Some("hello world".to_string()),
                    lines: None,
                    offsets: None,
                    section: None,
                    pointer: None,
                },
                content: Some("hello protocol".to_string()),
            }],
        );
        let output = store.apply(&env2).unwrap();
        assert_eq!(output.name, Name::Result);
        let result: ResultContentItem =
            serde_json::from_value(output.content[0].clone()).unwrap();
        assert_eq!(result.status, ResultStatus::Applied);
        assert_eq!(result.mode_used, Name::Diff);
        assert_eq!(get_body(&store, "test"), "<div>hello protocol</div>");
        assert_eq!(store.current_version("test"), Some(2));
    }

    #[test]
    fn test_version_conflict() {
        let mut store = ArtifactStore::new(10);
        store
            .apply(&full_envelope("test", 1, "content"))
            .unwrap();

        // version 6 requires stored version == 5, but stored is 1
        let bad_env = diff_envelope("test", 6, vec![]);
        assert!(store.apply(&bad_env).is_err());
    }

    #[test]
    fn test_rollback() {
        let mut store = ArtifactStore::new(10);
        store
            .apply(&full_envelope("test", 1, "version one"))
            .unwrap();
        store
            .apply(&full_envelope("test", 2, "version two"))
            .unwrap();

        let rolled = store.rollback("test", 1).unwrap();
        let item: FullContentItem =
            serde_json::from_value(rolled.content[0].clone()).unwrap();
        assert_eq!(item.body, "version one");
        assert_eq!(store.current_version("test"), Some(3));
    }

    #[test]
    fn test_full_returns_handle_with_sections() {
        let mut store = ArtifactStore::new(10);
        let sections = vec![
            SectionDef {
                id: "nav".to_string(),
                label: None,
                start_marker: None,
                end_marker: None,
            },
            SectionDef {
                id: "body".to_string(),
                label: None,
                start_marker: None,
                end_marker: None,
            },
        ];
        let env = full_envelope_with_sections("test", 1, "<html>content</html>", sections);
        let output = store.apply(&env).unwrap();

        assert_eq!(output.name, Name::Handle);
        assert_eq!(output.protocol, PROTOCOL_VERSION);
        assert_eq!(output.id, "test");
        assert_eq!(output.version, 1);

        let handle: HandleContentItem =
            serde_json::from_value(output.content[0].clone()).unwrap();
        assert_eq!(handle.sections.len(), 2);
        assert_eq!(handle.sections[0].id, "nav");
        assert_eq!(handle.sections[1].id, "body");
        assert!(output.operation.checksum.is_some());
    }

    #[test]
    fn test_result_checksum_matches_content() {
        let mut store = ArtifactStore::new(10);
        store
            .apply(&full_envelope("test", 1, "original"))
            .unwrap();

        let env = diff_envelope(
            "test",
            2,
            vec![DiffOp {
                op: OpType::Replace,
                target: Target {
                    search: Some("original".to_string()),
                    lines: None,
                    offsets: None,
                    section: None,
                    pointer: None,
                },
                content: Some("updated".to_string()),
            }],
        );
        let output = store.apply(&env).unwrap();
        let result: ResultContentItem =
            serde_json::from_value(output.content[0].clone()).unwrap();

        let expected_checksum = {
            let mut hasher = Sha256::new();
            hasher.update(b"updated");
            format!("sha256:{:x}", hasher.finalize())
        };
        assert_eq!(result.checksum.unwrap(), expected_checksum);
    }
}
