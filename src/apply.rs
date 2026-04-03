//! Stateless apply engine — pure function that transforms artifacts.
//!
//! `apply(artifact, operation) -> artifact`
//!
//! Takes an existing artifact (as a `name:"full"` envelope) and an operation
//! envelope, returns the new artifact state as a `name:"full"` envelope.
//! No state, no store, no side effects.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;

use crate::aap::{
    DiffOp, Envelope, FullContentItem, Include, ManifestContentItem, Name, OpType, Operation,
    SectionDef, SectionUpdate, TemplateContentItem, PROTOCOL_VERSION,
};
use crate::markers::{
    find_section_def, find_section_range, resolve_markers,
};

/// Extract the body string from a `name:"full"` envelope.
fn extract_body(envelope: &Envelope) -> Result<String> {
    let item: FullContentItem = serde_json::from_value(
        envelope
            .content
            .first()
            .context("full envelope: empty content array")?
            .clone(),
    )
    .context("full envelope: failed to parse content item")?;
    Ok(item.body)
}

/// Extract sections from a `name:"full"` envelope (if present).
fn extract_sections(envelope: &Envelope) -> Option<Vec<SectionDef>> {
    envelope
        .content
        .first()
        .and_then(|v| serde_json::from_value::<FullContentItem>(v.clone()).ok())
        .and_then(|item| item.sections)
}

/// Build a `name:"full"` output envelope from resolved content.
fn build_full_envelope(
    id: &str,
    version: u64,
    format: Option<&str>,
    body: String,
    sections: Option<Vec<SectionDef>>,
) -> Result<Envelope> {
    let content_item = FullContentItem { body, sections };
    Ok(Envelope {
        protocol: PROTOCOL_VERSION.to_string(),
        id: id.to_string(),
        version,
        name: Name::Full,
        operation: Operation {
            direction: "output".to_string(),
            format: format.map(|s| s.to_string()),
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
        },
        content: vec![
            serde_json::to_value(content_item).context("failed to serialize content item")?
        ],
    })
}

/// Stateless apply: `f(artifact, operation) → artifact`.
///
/// - `artifact`: current artifact state as a `name:"full"` envelope.
///   `None` for operations that don't need a base (`full`, `template`, `manifest`).
/// - `operation`: the operation envelope to apply.
///
/// Returns the new artifact state as a `name:"full"` envelope.
pub fn apply(artifact: Option<&Envelope>, operation: &Envelope) -> Result<Envelope> {
    let format = operation
        .operation
        .format
        .as_deref()
        .unwrap_or("text/html");

    let (body, sections) = match operation.name {
        Name::Full => {
            let body = extract_body(operation)?;
            let sections = extract_sections(operation);
            (body, sections)
        }
        Name::Diff => {
            let art = artifact.context("diff requires a base artifact")?;
            let base = extract_body(art)?;
            let sections = extract_sections(art);
            let ops: Vec<DiffOp> = operation
                .content
                .iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("diff: failed to parse content items")?;
            let body = apply_diff(&base, &ops, format, None)?;
            (body, sections)
        }
        Name::Section => {
            let art = artifact.context("section requires a base artifact")?;
            let base = extract_body(art)?;
            let sections = extract_sections(art);
            let updates: Vec<SectionUpdate> = operation
                .content
                .iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("section: failed to parse content items")?;
            let body = apply_section_update(&base, &updates, format, None)?;
            (body, sections)
        }
        Name::Template => {
            let item: TemplateContentItem = serde_json::from_value(
                operation
                    .content
                    .first()
                    .context("template: empty content array")?
                    .clone(),
            )
            .context("template: failed to parse content item")?;
            let body = fill_template(&item.template, &item.bindings);
            let sections = artifact.and_then(extract_sections);
            (body, sections)
        }
        Name::Composite => {
            let includes: Vec<Include> = operation
                .content
                .iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("composite: failed to parse content items")?;
            let body = resolve_composite(&includes, format)?;
            let sections = artifact.and_then(extract_sections);
            (body, sections)
        }
        Name::Manifest => {
            let item: ManifestContentItem = serde_json::from_value(
                operation
                    .content
                    .first()
                    .context("manifest: empty content array")?
                    .clone(),
            )
            .context("manifest: failed to parse content item")?;
            (item.skeleton, None)
        }
        Name::Handle | Name::Projection | Name::Intent | Name::Result | Name::Audit => {
            bail!(
                "control-plane operation '{:?}' does not produce artifact content",
                operation.name
            )
        }
    };

    build_full_envelope(
        &operation.id,
        operation.version,
        Some(format),
        body,
        sections,
    )
}

/// Apply diff operations sequentially to base content.
///
/// If any operation uses `pointer` targeting, the content is parsed as JSON
/// and pointer operations are applied on the parsed tree.
pub fn apply_diff(
    base: &str,
    operations: &[DiffOp],
    format: &str,
    sections: Option<&[SectionDef]>,
) -> Result<String> {
    let has_pointer = operations.iter().any(|op| op.target.pointer.is_some());

    if has_pointer {
        return apply_diff_with_pointers(base, operations, format);
    }

    let mut result = base.to_string();

    for (i, op) in operations.iter().enumerate() {
        let (start, end) = find_target_range(&result, &op.target, format, sections)
            .with_context(|| format!("operation {i}: target not found"))?;

        match op.op {
            OpType::Replace => {
                let content = op.content.as_deref().unwrap_or("");
                result = format!("{}{}{}", &result[..start], content, &result[end..]);
            }
            OpType::Delete => {
                result = format!("{}{}", &result[..start], &result[end..]);
            }
            OpType::InsertBefore => {
                let content = op.content.as_deref().unwrap_or("");
                result = format!("{}{}{}", &result[..start], content, &result[start..]);
            }
            OpType::InsertAfter => {
                let content = op.content.as_deref().unwrap_or("");
                result = format!("{}{}{}", &result[..end], content, &result[end..]);
            }
        }
    }

    Ok(result)
}

/// Apply diff operations that use JSON Pointer targeting.
fn apply_diff_with_pointers(base: &str, operations: &[DiffOp], _format: &str) -> Result<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(base).context("pointer targeting requires valid JSON content")?;

    for (i, op) in operations.iter().enumerate() {
        if let Some(pointer) = &op.target.pointer {
            apply_pointer_op(&mut value, pointer, op)
                .with_context(|| format!("operation {i}: pointer op failed"))?;
        } else {
            bail!("operation {i}: mixing pointer and non-pointer targets in the same batch is not supported");
        }
    }

    serde_json::to_string_pretty(&value).context("failed to re-serialize JSON")
}

/// Apply a single pointer-targeted operation on a parsed JSON value.
fn apply_pointer_op(root: &mut serde_json::Value, pointer: &str, op: &DiffOp) -> Result<()> {
    match op.op {
        OpType::Replace => {
            let content = op.content.as_deref().context("replace requires content")?;
            let new_val: serde_json::Value =
                serde_json::from_str(content).context("content must be valid JSON")?;
            let target = root
                .pointer_mut(pointer)
                .with_context(|| format!("pointer not found: {pointer}"))?;
            *target = new_val;
        }
        OpType::Delete => {
            let (parent_ptr, key) = split_pointer(pointer).context("cannot delete root")?;
            let parent = root
                .pointer_mut(&parent_ptr)
                .with_context(|| format!("parent not found: {parent_ptr}"))?;
            remove_child(parent, &key)?;
        }
        OpType::InsertBefore | OpType::InsertAfter => {
            let content = op.content.as_deref().context("insert requires content")?;
            let new_val: serde_json::Value =
                serde_json::from_str(content).context("content must be valid JSON")?;
            let (parent_ptr, key) = split_pointer(pointer).context("cannot insert at root")?;
            let parent = root
                .pointer_mut(&parent_ptr)
                .with_context(|| format!("parent not found: {parent_ptr}"))?;
            let arr = parent
                .as_array_mut()
                .context("insert_before/insert_after require array parent")?;
            let index: usize = key
                .parse()
                .context("insert_before/insert_after require numeric array index")?;
            let insert_at = if op.op == OpType::InsertAfter {
                index + 1
            } else {
                index
            };
            arr.insert(insert_at, new_val);
        }
    }
    Ok(())
}

/// Split a JSON Pointer into parent path and final key.
/// `/a/b/c` → (`/a/b`, `c`)
/// `/a` → (``, `a`)
fn split_pointer(pointer: &str) -> Result<(String, String)> {
    if pointer.is_empty() || !pointer.starts_with('/') {
        bail!("invalid JSON Pointer: {pointer:?}");
    }
    match pointer.rfind('/') {
        Some(0) => Ok(("".to_string(), pointer[1..].to_string())),
        Some(pos) => Ok((pointer[..pos].to_string(), pointer[pos + 1..].to_string())),
        None => bail!("invalid JSON Pointer: {pointer:?}"),
    }
}

/// Remove a child from a JSON object or array.
fn remove_child(parent: &mut serde_json::Value, key: &str) -> Result<()> {
    // Unescape RFC 6901
    let unescaped = key.replace("~1", "/").replace("~0", "~");

    if let Some(obj) = parent.as_object_mut() {
        if obj.remove(&unescaped).is_none() {
            bail!("key not found: {unescaped}");
        }
    } else if let Some(arr) = parent.as_array_mut() {
        let index: usize = unescaped
            .parse()
            .with_context(|| format!("array index expected, got: {unescaped}"))?;
        if index >= arr.len() {
            bail!("array index out of bounds: {index}");
        }
        arr.remove(index);
    } else {
        bail!("parent is neither object nor array");
    }
    Ok(())
}

/// Replace section content, preserving markers and other sections.
pub fn apply_section_update(
    base: &str,
    updates: &[SectionUpdate],
    format: &str,
    sections: Option<&[SectionDef]>,
) -> Result<String> {
    let mut result = base.to_string();

    for update in updates {
        let section_def = find_section_def(sections, &update.id);
        let (start_marker, _end_marker) = resolve_markers(&update.id, format, section_def)
            .with_context(|| format!("cannot resolve markers for section: {}", update.id))?;
        let (content_start, content_end) =
            find_section_range(&result, &update.id, format, section_def)
                .with_context(|| format!("section not found: {}", update.id))?;

        // Rebuild: everything up to and including start marker, new content, then from end marker
        let marker_end_pos = result[..content_start]
            .rfind(&start_marker)
            .map(|pos| pos + start_marker.len())
            .unwrap_or(content_start);
        let before = &result[..marker_end_pos];
        let after = &result[content_end..];
        result = format!("{before}\n{}\n{after}", update.content);
    }

    Ok(result)
}

/// Simple Mustache-subset template filling (variable substitution).
pub fn fill_template(template: &str, bindings: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in bindings {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        // Unescaped triple-brace
        result = result.replace(&format!("{{{{{{{key}}}}}}}"), &val_str);
        // Regular double-brace
        result = result.replace(&format!("{{{{{key}}}}}"), &val_str);
    }
    result
}

/// Assemble content from inline include items.
///
/// Only inline `content` items are supported. References (`ref`, `uri`) must
/// be pre-resolved by the caller before invoking the stateless apply engine.
pub fn resolve_composite(includes: &[Include], _format: &str) -> Result<String> {
    let mut parts = Vec::new();

    for inc in includes {
        if let Some(content) = &inc.content {
            parts.push(content.clone());
        } else if inc.reference.is_some() {
            bail!("composite ref must be pre-resolved before calling apply — the apply engine is stateless");
        } else if inc.uri.is_some() {
            bail!("composite uri must be pre-resolved before calling apply — the apply engine is stateless");
        } else {
            bail!("include has no ref, uri, or content");
        }
    }

    Ok(parts.join("\n"))
}

/// Assemble a manifest by stitching section results into the skeleton.
///
/// Each entry in `section_results` maps a section ID to its generated content.
/// The content is inserted between the corresponding section markers in the skeleton.
pub fn assemble_manifest(
    skeleton: &str,
    section_results: &HashMap<String, String>,
    format: &str,
    sections: Option<&[SectionDef]>,
) -> Result<String> {
    let mut result = skeleton.to_string();
    for (section_id, content) in section_results {
        let section_def = find_section_def(sections, section_id);
        let (start_marker, _end_marker) = resolve_markers(section_id, format, section_def)
            .with_context(|| format!("cannot resolve markers for section: {section_id}"))?;
        let (content_start, content_end) =
            find_section_range(&result, section_id, format, section_def)
                .with_context(|| format!("section marker not found in skeleton: {section_id}"))?;

        let marker_end_pos = result[..content_start]
            .rfind(&start_marker)
            .map(|pos| pos + start_marker.len())
            .unwrap_or(content_start);
        let before = &result[..marker_end_pos];
        let after = &result[content_end..];
        result = format!("{before}\n{content}\n{after}");
    }
    Ok(result)
}

/// Find the byte range targeted by a diff operation's target.
fn find_target_range(
    content: &str,
    target: &crate::aap::Target,
    format: &str,
    sections: Option<&[SectionDef]>,
) -> Result<(usize, usize)> {
    if let Some(search) = &target.search {
        let idx = content
            .find(search.as_str())
            .with_context(|| format!("search target not found: {search:?}"))?;
        Ok((idx, idx + search.len()))
    } else if let Some(offsets) = &target.offsets {
        Ok((offsets[0] as usize, offsets[1] as usize))
    } else if let Some(lines) = &target.lines {
        let content_lines: Vec<&str> = content.split('\n').collect();
        let start_line = (lines[0] as usize).saturating_sub(1);
        let end_line = lines[1] as usize;
        let start = content_lines[..start_line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>();
        let end = content_lines[..end_line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            .saturating_sub(1);
        Ok((start, end))
    } else if let Some(section) = &target.section {
        let section_def = find_section_def(sections, section);
        find_section_range(content, section, format, section_def)
    } else {
        bail!("target has no addressing mode")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aap::{DiffOp, OpType, Target};

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

    #[test]
    fn test_apply_full() {
        let op = full_envelope("test", 1, "<div>hello</div>");
        let result = apply(None, &op).unwrap();
        assert_eq!(result.name, Name::Full);
        assert_eq!(extract_body(&result).unwrap(), "<div>hello</div>");
    }

    #[test]
    fn test_apply_diff() {
        let artifact = full_envelope("test", 1, "<div>old value</div>");
        let op = Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: "test".to_string(),
            version: 2,
            name: Name::Diff,
            operation: make_operation("text/html"),
            content: vec![serde_json::to_value(DiffOp {
                op: OpType::Replace,
                target: Target {
                    search: Some("old value".to_string()),
                    lines: None,
                    offsets: None,
                    section: None,
                    pointer: None,
                },
                content: Some("new value".to_string()),
            })
            .unwrap()],
        };
        let result = apply(Some(&artifact), &op).unwrap();
        assert_eq!(result.name, Name::Full);
        assert_eq!(extract_body(&result).unwrap(), "<div>new value</div>");
    }

    #[test]
    fn test_apply_diff_preserves_sections() {
        let artifact = Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: "test".to_string(),
            version: 1,
            name: Name::Full,
            operation: make_operation("text/html"),
            content: vec![serde_json::json!({
                "body": "<div>old</div>",
                "sections": [{"id": "main"}]
            })],
        };
        let op = Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: "test".to_string(),
            version: 2,
            name: Name::Diff,
            operation: make_operation("text/html"),
            content: vec![serde_json::to_value(DiffOp {
                op: OpType::Replace,
                target: Target {
                    search: Some("old".to_string()),
                    lines: None,
                    offsets: None,
                    section: None,
                    pointer: None,
                },
                content: Some("new".to_string()),
            })
            .unwrap()],
        };
        let result = apply(Some(&artifact), &op).unwrap();
        let sections = extract_sections(&result).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "main");
    }

    #[test]
    fn test_apply_diff_without_artifact_fails() {
        let op = Envelope {
            protocol: PROTOCOL_VERSION.to_string(),
            id: "test".to_string(),
            version: 2,
            name: Name::Diff,
            operation: make_operation("text/html"),
            content: vec![],
        };
        assert!(apply(None, &op).is_err());
    }

    #[test]
    fn test_apply_diff_search_replace() {
        let base = "<div>old value</div>";
        let ops = vec![DiffOp {
            op: OpType::Replace,
            target: Target {
                search: Some("old value".to_string()),
                lines: None,
                offsets: None,
                section: None,
                pointer: None,
            },
            content: Some("new value".to_string()),
        }];
        let result = apply_diff(base, &ops, "text/html", None).unwrap();
        assert_eq!(result, "<div>new value</div>");
    }

    #[test]
    fn test_apply_diff_delete() {
        let base = "keep this, remove this, keep that";
        let ops = vec![DiffOp {
            op: OpType::Delete,
            target: Target {
                search: Some(", remove this".to_string()),
                lines: None,
                offsets: None,
                section: None,
                pointer: None,
            },
            content: None,
        }];
        let result = apply_diff(base, &ops, "text/html", None).unwrap();
        assert_eq!(result, "keep this, keep that");
    }

    #[test]
    fn test_apply_section_update() {
        let base = "before\n<aap:section id=\"stats\">\nold stats\n</aap:section>\nafter";
        let updates = vec![SectionUpdate {
            id: "stats".to_string(),
            content: "new stats".to_string(),
        }];
        let result = apply_section_update(base, &updates, "text/html", None).unwrap();
        assert!(result.contains("new stats"));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn test_apply_section_update_python() {
        let base = "import os\n<aap:section id=\"imports\">\nold imports\n</aap:section>\ncode";
        let updates = vec![SectionUpdate {
            id: "imports".to_string(),
            content: "import sys\nimport json".to_string(),
        }];
        let result = apply_section_update(base, &updates, "text/x-python", None).unwrap();
        assert!(result.contains("import sys\nimport json"));
        assert!(result.contains("import os"));
        assert!(result.contains("code"));
    }

    #[test]
    fn test_apply_section_update_javascript() {
        let base = "const a = 1;\n<aap:section id=\"utils\">\nold utils\n</aap:section>\nconst b = 2;";
        let updates = vec![SectionUpdate {
            id: "utils".to_string(),
            content: "function helper() {}".to_string(),
        }];
        let result =
            apply_section_update(base, &updates, "application/javascript", None).unwrap();
        assert!(result.contains("function helper() {}"));
        assert!(result.contains("const a = 1;"));
        assert!(result.contains("const b = 2;"));
    }

    #[test]
    fn test_fill_template() {
        let template = "<h1>{{title}}</h1><p>{{{body}}}</p>";
        let mut bindings = HashMap::new();
        bindings.insert(
            "title".to_string(),
            serde_json::Value::String("Hello".to_string()),
        );
        bindings.insert(
            "body".to_string(),
            serde_json::Value::String("<b>World</b>".to_string()),
        );
        let result = fill_template(template, &bindings);
        assert_eq!(result, "<h1>Hello</h1><p><b>World</b></p>");
    }

    #[test]
    fn test_assemble_manifest() {
        let skeleton =
            "<html><aap:section id=\"nav\"></aap:section><main><aap:section id=\"body\"></aap:section></main></html>";
        let mut sections = HashMap::new();
        sections.insert("nav".to_string(), "<nav>Home</nav>".to_string());
        sections.insert("body".to_string(), "<h1>Hello</h1>".to_string());
        let result = assemble_manifest(skeleton, &sections, "text/html", None).unwrap();
        assert!(result.contains("<nav>Home</nav>"));
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<aap:section id=\"nav\">"));
        assert!(result.contains("</aap:section>"));
    }

    #[test]
    fn test_assemble_manifest_python() {
        let skeleton = "<aap:section id=\"header\"></aap:section>\n<aap:section id=\"body\"></aap:section>";
        let mut sections = HashMap::new();
        sections.insert("header".to_string(), "import os".to_string());
        sections.insert("body".to_string(), "print('hello')".to_string());
        let result =
            assemble_manifest(skeleton, &sections, "text/x-python", None).unwrap();
        assert!(result.contains("import os"));
        assert!(result.contains("print('hello')"));
    }

    #[test]
    fn test_diff_with_section_target_python() {
        let base = "<aap:section id=\"config\">\nold_value = 1\n</aap:section>\ncode";
        let ops = vec![DiffOp {
            op: OpType::Replace,
            target: Target {
                section: Some("config".to_string()),
                search: None,
                lines: None,
                offsets: None,
                pointer: None,
            },
            content: Some("new_value = 2".to_string()),
        }];
        let result = apply_diff(base, &ops, "text/x-python", None).unwrap();
        assert!(result.contains("new_value = 2"));
        assert!(!result.contains("old_value"));
    }

    #[test]
    fn test_pointer_replace() {
        let base = r#"{"name": "Alice", "age": 30}"#;
        let ops = vec![DiffOp {
            op: OpType::Replace,
            target: Target {
                pointer: Some("/name".to_string()),
                search: None,
                lines: None,
                offsets: None,
                section: None,
            },
            content: Some(r#""Bob""#.to_string()),
        }];
        let result = apply_diff(base, &ops, "application/json", None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Bob");
        assert_eq!(parsed["age"], 30);
    }

    #[test]
    fn test_pointer_delete() {
        let base = r#"{"name": "Alice", "age": 30, "temp": true}"#;
        let ops = vec![DiffOp {
            op: OpType::Delete,
            target: Target {
                pointer: Some("/temp".to_string()),
                search: None,
                lines: None,
                offsets: None,
                section: None,
            },
            content: None,
        }];
        let result = apply_diff(base, &ops, "application/json", None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("temp").is_none());
        assert_eq!(parsed["name"], "Alice");
    }

    #[test]
    fn test_pointer_insert_after_array() {
        let base = r#"{"items": ["a", "b", "c"]}"#;
        let ops = vec![DiffOp {
            op: OpType::InsertAfter,
            target: Target {
                pointer: Some("/items/1".to_string()),
                search: None,
                lines: None,
                offsets: None,
                section: None,
            },
            content: Some(r#""x""#.to_string()),
        }];
        let result = apply_diff(base, &ops, "application/json", None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let items: Vec<&str> = parsed["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(items, vec!["a", "b", "x", "c"]);
    }

    #[test]
    fn test_pointer_nested_replace() {
        let base = r#"{"db": {"host": "localhost", "port": 5432}}"#;
        let ops = vec![DiffOp {
            op: OpType::Replace,
            target: Target {
                pointer: Some("/db/host".to_string()),
                search: None,
                lines: None,
                offsets: None,
                section: None,
            },
            content: Some(r#""prod.example.com""#.to_string()),
        }];
        let result = apply_diff(base, &ops, "application/json", None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["db"]["host"], "prod.example.com");
        assert_eq!(parsed["db"]["port"], 5432);
    }
}
