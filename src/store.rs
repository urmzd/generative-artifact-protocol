//! Versioned artifact store — maintains artifact history for diff/section
//! application and rollback support.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};

use crate::apply;
use crate::aap::{Envelope, Name};

/// Entry in the version history.
#[derive(Debug, Clone)]
struct VersionEntry {
    version: u64,
    content: String,
}

/// In-memory versioned artifact store.
///
/// Maintains a history of resolved content for each artifact ID,
/// enabling diff application, version chain integrity checks, and rollback.
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

    /// Get the current content for an artifact.
    pub fn get(&self, id: &str) -> Option<&str> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| e.content.as_str())
    }

    /// Get the current version number for an artifact.
    pub fn current_version(&self, id: &str) -> Option<u64> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| e.version)
    }

    /// Build a content map for use with `apply::resolve`.
    pub fn content_map(&self) -> HashMap<String, String> {
        self.history
            .iter()
            .filter_map(|(id, v)| v.last().map(|e| (id.clone(), e.content.clone())))
            .collect()
    }

    /// Apply an envelope, resolving its content and storing the result.
    ///
    /// Enforces version chain integrity: for non-full names, the stored
    /// version must equal `envelope.version - 1` (implicit base_version).
    pub fn apply(&mut self, envelope: &Envelope) -> Result<String> {
        // Version chain check for incremental operations
        if envelope.name != Name::Full {
            if let Some(current) = self.current_version(&envelope.id) {
                if current != envelope.version - 1 {
                    bail!(
                        "version conflict: stored version={current}, envelope version={}, expected stored=={}",
                        envelope.version,
                        envelope.version - 1
                    );
                }
            } else {
                bail!(
                    "no base content for artifact '{}' — full mode required first",
                    envelope.id
                );
            }
        }

        let store_map = self.content_map();
        let resolved = apply::resolve(envelope, &store_map)?;

        let entries = self.history.entry(envelope.id.clone()).or_default();
        entries.push(VersionEntry {
            version: envelope.version,
            content: resolved.clone(),
        });

        // Trim old versions
        while entries.len() > self.max_history {
            entries.remove(0);
        }

        Ok(resolved)
    }

    /// Roll back to a previous version. Returns the restored content.
    pub fn rollback(&mut self, id: &str, target_version: u64) -> Result<String> {
        let entries = self
            .history
            .get_mut(id)
            .context("artifact not found")?;

        let idx = entries
            .iter()
            .position(|e| e.version == target_version)
            .with_context(|| format!("version {target_version} not in history"))?;

        let content = entries[idx].content.clone();
        let new_version = entries.last().map(|e| e.version).unwrap_or(0) + 1;

        entries.push(VersionEntry {
            version: new_version,
            content: content.clone(),
        });

        while entries.len() > self.max_history {
            entries.remove(0);
        }

        Ok(content)
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

    #[test]
    fn test_full_then_diff() {
        let mut store = ArtifactStore::new(10);

        let env1 = full_envelope("test", 1, "<div>hello world</div>");
        let content = store.apply(&env1).unwrap();
        assert_eq!(content, "<div>hello world</div>");

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
        let content = store.apply(&env2).unwrap();
        assert_eq!(content, "<div>hello protocol</div>");
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
        assert_eq!(rolled, "version one");
        assert_eq!(store.current_version("test"), Some(3));
    }
}
