//! Format-aware section marker resolution.
//!
//! Maps MIME types to the appropriate section marker syntax and provides
//! centralized marker lookup for the apply engine.

use anyhow::{bail, Context, Result};

use crate::aap::SectionDef;

/// Marker style families derived from MIME type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStyle {
    /// `<!-- section:id -->` / `<!-- /section:id -->`
    HtmlComment,
    /// `// #region id` / `// #endregion id`
    CStyleRegion,
    /// `# region id` / `# endregion id`
    HashRegion,
    /// Format does not support text-based section markers.
    Unsupported,
}

/// Determine the marker style for a given MIME type.
pub fn marker_style_for_format(format: &str) -> MarkerStyle {
    match format {
        // HTML family
        "text/html" | "text/markdown" | "image/svg+xml" => MarkerStyle::HtmlComment,

        // C-style comment languages
        "application/javascript"
        | "text/javascript"
        | "text/typescript"
        | "text/x-rust"
        | "text/x-go"
        | "text/x-c"
        | "text/x-c++"
        | "text/x-java"
        | "text/x-csharp"
        | "text/x-scala"
        | "text/x-kotlin"
        | "text/x-swift"
        | "text/css" => MarkerStyle::CStyleRegion,

        // Hash-comment languages
        "text/x-python"
        | "text/x-ruby"
        | "application/x-sh"
        | "text/x-shellscript"
        | "application/x-toml"
        | "text/x-yaml"
        | "application/yaml"
        | "text/x-perl"
        | "text/x-r" => MarkerStyle::HashRegion,

        // Structured formats — use JSON Pointer instead
        "application/json" => MarkerStyle::Unsupported,

        // Fallback: XML-family gets HTML comments, other text gets HTML comments
        other => {
            if other.ends_with("+xml") {
                MarkerStyle::HtmlComment
            } else if other.starts_with("text/") {
                MarkerStyle::HtmlComment
            } else {
                MarkerStyle::Unsupported
            }
        }
    }
}

/// Build start and end markers for a section ID given a marker style.
fn markers_for_style(style: MarkerStyle, section_id: &str) -> Result<(String, String)> {
    match style {
        MarkerStyle::HtmlComment => Ok((
            format!("<!-- section:{section_id} -->"),
            format!("<!-- /section:{section_id} -->"),
        )),
        MarkerStyle::CStyleRegion => Ok((
            format!("// #region {section_id}"),
            format!("// #endregion {section_id}"),
        )),
        MarkerStyle::HashRegion => Ok((
            format!("# region {section_id}"),
            format!("# endregion {section_id}"),
        )),
        MarkerStyle::Unsupported => {
            bail!("format does not support text-based section markers; use JSON Pointer addressing instead")
        }
    }
}

/// Resolve section markers for a given section ID and format.
///
/// If the `section_def` provides explicit `start_marker` and `end_marker`,
/// those are used (user override). Otherwise, markers are derived from the
/// MIME type.
pub fn resolve_markers(
    section_id: &str,
    format: &str,
    section_def: Option<&SectionDef>,
) -> Result<(String, String)> {
    // Explicit override wins
    if let Some(def) = section_def {
        if let (Some(start), Some(end)) = (&def.start_marker, &def.end_marker) {
            return Ok((start.clone(), end.clone()));
        }
    }

    let style = marker_style_for_format(format);
    markers_for_style(style, section_id)
}

/// Find the byte range of a section's content within a string.
///
/// Returns `(content_start, content_end)` — the byte offsets of the content
/// between the start and end markers (exclusive of markers themselves).
pub fn find_section_range(
    content: &str,
    section_id: &str,
    format: &str,
    section_def: Option<&SectionDef>,
) -> Result<(usize, usize)> {
    let (start_marker, end_marker) = resolve_markers(section_id, format, section_def)?;
    let si = content
        .find(&start_marker)
        .with_context(|| format!("start marker not found for section: {section_id}"))?;
    let ei = content
        .find(&end_marker)
        .with_context(|| format!("end marker not found for section: {section_id}"))?;
    Ok((si + start_marker.len(), ei))
}

/// Find the byte range of a section including its markers.
///
/// Returns `(marker_start, marker_end)` — the byte offsets that include both
/// the start marker, content, and end marker.
pub fn find_section_range_inclusive(
    content: &str,
    section_id: &str,
    format: &str,
    section_def: Option<&SectionDef>,
) -> Result<(usize, usize)> {
    let (start_marker, end_marker) = resolve_markers(section_id, format, section_def)?;
    let si = content
        .find(&start_marker)
        .with_context(|| format!("start marker not found for section: {section_id}"))?;
    let ei = content
        .find(&end_marker)
        .with_context(|| format!("end marker not found for section: {section_id}"))?;
    Ok((si, ei + end_marker.len()))
}

/// Look up a `SectionDef` by ID from an optional list.
pub fn find_section_def<'a>(
    sections: Option<&'a [SectionDef]>,
    section_id: &str,
) -> Option<&'a SectionDef> {
    sections?.iter().find(|s| s.id == section_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_markers() {
        let (start, end) = resolve_markers("nav", "text/html", None).unwrap();
        assert_eq!(start, "<!-- section:nav -->");
        assert_eq!(end, "<!-- /section:nav -->");
    }

    #[test]
    fn test_javascript_markers() {
        let (start, end) = resolve_markers("utils", "application/javascript", None).unwrap();
        assert_eq!(start, "// #region utils");
        assert_eq!(end, "// #endregion utils");
    }

    #[test]
    fn test_python_markers() {
        let (start, end) = resolve_markers("imports", "text/x-python", None).unwrap();
        assert_eq!(start, "# region imports");
        assert_eq!(end, "# endregion imports");
    }

    #[test]
    fn test_json_unsupported() {
        let result = resolve_markers("data", "application/json", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_explicit_override() {
        let def = SectionDef {
            id: "custom".to_string(),
            label: None,
            start_marker: Some("/* BEGIN custom */".to_string()),
            end_marker: Some("/* END custom */".to_string()),
        };
        let (start, end) = resolve_markers("custom", "application/json", Some(&def)).unwrap();
        assert_eq!(start, "/* BEGIN custom */");
        assert_eq!(end, "/* END custom */");
    }

    #[test]
    fn test_xml_family_fallback() {
        let style = marker_style_for_format("application/xhtml+xml");
        assert_eq!(style, MarkerStyle::HtmlComment);
    }

    #[test]
    fn test_unknown_text_fallback() {
        let style = marker_style_for_format("text/x-unknown");
        assert_eq!(style, MarkerStyle::HtmlComment);
    }

    #[test]
    fn test_find_section_range() {
        let content = "before\n<!-- section:stats -->\nold stats\n<!-- /section:stats -->\nafter";
        let (start, end) = find_section_range(content, "stats", "text/html", None).unwrap();
        assert_eq!(&content[start..end], "\nold stats\n");
    }

    #[test]
    fn test_find_section_range_python() {
        let content = "import os\n# region imports\nimport sys\n# endregion imports\ncode";
        let (start, end) = find_section_range(content, "imports", "text/x-python", None).unwrap();
        assert_eq!(&content[start..end], "\nimport sys\n");
    }
}
