//! Versioned artifact store — stateful layer over the stateless apply engine.
//!
//! Manages version chains, history. The apply engine is a pure function;
//! the store provides persistence on top.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::aap::{Envelope, Name, SynthesizeContentItem};
use crate::apply;

/// Entry in the version history.
#[derive(Debug, Clone)]
struct VersionEntry {
    version: u64,
    envelope: Envelope,
}

/// In-memory versioned artifact store.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    history: HashMap<String, Vec<VersionEntry>>,
    max_history: usize,
}

impl ArtifactStore {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: HashMap::new(),
            max_history,
        }
    }

    pub fn get(&self, id: &str) -> Option<&Envelope> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| &e.envelope)
    }

    pub fn current_version(&self, id: &str) -> Option<u64> {
        self.history
            .get(id)
            .and_then(|v| v.last())
            .map(|e| e.version)
    }

    /// Apply an envelope, store the resolved artifact, return (artifact, handle).
    pub fn apply(&mut self, operation: &Envelope) -> Result<(Envelope, Envelope)> {
        if operation.name != Name::Synthesize {
            if let Some(current) = self.current_version(&operation.id) {
                if current != operation.version - 1 {
                    bail!(
                        "version conflict: stored={current}, envelope={}, expected={}",
                        operation.version,
                        operation.version - 1
                    );
                }
            } else {
                bail!(
                    "no base content for artifact '{}' — synthesize required first",
                    operation.id
                );
            }
        }

        let artifact = self.get(&operation.id);
        let (resolved, handle) = apply::apply(artifact, operation)?;

        let entries = self.history.entry(operation.id.clone()).or_default();
        entries.push(VersionEntry {
            version: operation.version,
            envelope: resolved.clone(),
        });

        while entries.len() > self.max_history {
            entries.remove(0);
        }

        Ok((resolved, handle))
    }

    /// Compute sha256 checksum of the current artifact body.
    pub fn checksum(&self, id: &str) -> Result<String> {
        let env = self.get(id).context("artifact not found")?;
        let item: SynthesizeContentItem =
            serde_json::from_value(env.content[0].clone()).context("parse synthesize item")?;
        let mut hasher = Sha256::new();
        hasher.update(item.body.as_bytes());
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    pub fn rollback(&mut self, id: &str, target_version: u64) -> Result<Envelope> {
        let entries = self.history.get_mut(id).context("artifact not found")?;
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

    fn make_op(format: &str) -> Operation {
        Operation {
            direction: "output".to_string(),
            format: Some(format.to_string()),
            encoding: None,
            content_encoding: None,
            token_budget: None,
            tokens_used: None,
            checksum: None,
            created_at: None,
            updated_at: None,
            state: None,
            state_changed_at: None,
        }
    }

    fn synthesize_env(id: &str, version: u64, body: &str) -> Envelope {
        Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: id.to_string(),
            version,
            name: Name::Synthesize,
            operation: make_op("text/html"),
            content: vec![serde_json::json!({ "body": body })],
        }
    }

    fn edit_env(id: &str, version: u64, ops: Vec<DiffOp>) -> Envelope {
        Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: id.to_string(),
            version,
            name: Name::Edit,
            operation: make_op("text/html"),
            content: ops
                .iter()
                .map(|op| serde_json::to_value(op).unwrap())
                .collect(),
        }
    }

    fn get_body(store: &ArtifactStore, id: &str) -> String {
        let env = store.get(id).unwrap();
        let item: SynthesizeContentItem =
            serde_json::from_value(env.content[0].clone()).unwrap();
        item.body
    }

    #[test]
    fn test_synthesize_then_edit_by_id() {
        let mut store = ArtifactStore::new(10);

        let body = r#"<aap:target id="msg">hello</aap:target>"#;
        store.apply(&synthesize_env("test", 1, body)).unwrap();
        assert_eq!(get_body(&store, "test"), body);

        let env2 = edit_env(
            "test",
            2,
            vec![DiffOp {
                op: OpType::Replace,
                target: Target::Id("msg".to_string()),
                content: Some("world".to_string()),
            }],
        );
        store.apply(&env2).unwrap();
        assert!(get_body(&store, "test").contains("world"));
        assert_eq!(store.current_version("test"), Some(2));
    }

    #[test]
    fn test_version_conflict() {
        let mut store = ArtifactStore::new(10);
        store.apply(&synthesize_env("test", 1, "content")).unwrap();
        let bad = edit_env("test", 6, vec![]);
        assert!(store.apply(&bad).is_err());
    }

    #[test]
    fn test_rollback() {
        let mut store = ArtifactStore::new(10);
        store.apply(&synthesize_env("test", 1, "v1")).unwrap();
        store.apply(&synthesize_env("test", 2, "v2")).unwrap();

        let rolled = store.rollback("test", 1).unwrap();
        let item: SynthesizeContentItem =
            serde_json::from_value(rolled.content[0].clone()).unwrap();
        assert_eq!(item.body, "v1");
        assert_eq!(store.current_version("test"), Some(3));
    }

    #[test]
    fn test_checksum() {
        let mut store = ArtifactStore::new(10);
        store.apply(&synthesize_env("test", 1, "hello")).unwrap();
        let cs = store.checksum("test").unwrap();
        assert!(cs.starts_with("sha256:"));
    }

    #[test]
    fn test_targets_preserved() {
        let mut store = ArtifactStore::new(10);
        let env = Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: "test".to_string(),
            version: 1,
            name: Name::Synthesize,
            operation: make_op("text/html"),
            content: vec![serde_json::json!({
                "body": r#"<aap:target id="x">old</aap:target>"#,
                "targets": [{"id": "x"}]
            })],
        };
        store.apply(&env).unwrap();

        let edit = edit_env(
            "test",
            2,
            vec![DiffOp {
                op: OpType::Replace,
                target: Target::Id("x".to_string()),
                content: Some("new".to_string()),
            }],
        );
        store.apply(&edit).unwrap();

        let resolved = store.get("test").unwrap();
        let item: SynthesizeContentItem =
            serde_json::from_value(resolved.content[0].clone()).unwrap();
        assert!(item.body.contains("new"));
        assert_eq!(item.targets.unwrap()[0].id, "x");
    }
}
