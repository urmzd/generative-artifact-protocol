"""
Format-aware section marker resolution.

Maps MIME types to the appropriate section marker syntax and provides
centralized marker lookup for the apply engine.
"""
from __future__ import annotations

from enum import Enum

from aap.aap import SectionDef


class MarkerStyle(Enum):
    """Marker style families derived from MIME type."""
    HTML_COMMENT = "html_comment"
    C_STYLE_REGION = "c_style_region"
    HASH_REGION = "hash_region"
    UNSUPPORTED = "unsupported"


# MIME types mapped to C-style region markers
_C_STYLE_FORMATS = frozenset({
    "application/javascript",
    "text/javascript",
    "text/typescript",
    "text/x-rust",
    "text/x-go",
    "text/x-c",
    "text/x-c++",
    "text/x-java",
    "text/x-csharp",
    "text/x-scala",
    "text/x-kotlin",
    "text/x-swift",
    "text/css",
})

# MIME types mapped to hash-comment region markers
_HASH_FORMATS = frozenset({
    "text/x-python",
    "text/x-ruby",
    "application/x-sh",
    "text/x-shellscript",
    "application/x-toml",
    "text/x-yaml",
    "application/yaml",
    "text/x-perl",
    "text/x-r",
})

# MIME types mapped to HTML comment markers
_HTML_COMMENT_FORMATS = frozenset({
    "text/html",
    "text/markdown",
    "image/svg+xml",
})


def marker_style_for_format(format: str) -> MarkerStyle:
    """Determine the marker style for a given MIME type."""
    if format in _HTML_COMMENT_FORMATS:
        return MarkerStyle.HTML_COMMENT
    if format in _C_STYLE_FORMATS:
        return MarkerStyle.C_STYLE_REGION
    if format in _HASH_FORMATS:
        return MarkerStyle.HASH_REGION
    if format == "application/json":
        return MarkerStyle.UNSUPPORTED
    # Fallbacks
    if format.endswith("+xml"):
        return MarkerStyle.HTML_COMMENT
    if format.startswith("text/"):
        return MarkerStyle.HTML_COMMENT
    return MarkerStyle.UNSUPPORTED


def _markers_for_style(style: MarkerStyle, section_id: str) -> tuple[str, str]:
    """Build start and end markers for a section ID given a marker style."""
    if style == MarkerStyle.HTML_COMMENT:
        return f"<!-- section:{section_id} -->", f"<!-- /section:{section_id} -->"
    if style == MarkerStyle.C_STYLE_REGION:
        return f"// #region {section_id}", f"// #endregion {section_id}"
    if style == MarkerStyle.HASH_REGION:
        return f"# region {section_id}", f"# endregion {section_id}"
    raise ValueError(
        f"format does not support text-based section markers; "
        f"use JSON Pointer addressing instead"
    )


def resolve_markers(
    section_id: str,
    format: str,
    section_def: SectionDef | None = None,
) -> tuple[str, str]:
    """Resolve section markers for a given section ID and format.

    If the section_def provides explicit start_marker and end_marker,
    those are used (user override). Otherwise, markers are derived from
    the MIME type.
    """
    if section_def is not None:
        if section_def.start_marker and section_def.end_marker:
            return section_def.start_marker, section_def.end_marker

    style = marker_style_for_format(format)
    return _markers_for_style(style, section_id)


def find_section_range(
    content: str,
    section_id: str,
    format: str,
    section_def: SectionDef | None = None,
) -> tuple[int, int]:
    """Find the byte range of a section's content within a string.

    Returns (content_start, content_end) — the character offsets of the
    content between the start and end markers.
    """
    start_marker, end_marker = resolve_markers(section_id, format, section_def)
    si = content.find(start_marker)
    ei = content.find(end_marker)
    if si == -1 or ei == -1:
        raise ValueError(f"Section markers not found: {section_id}")
    return si + len(start_marker), ei


def find_section_range_inclusive(
    content: str,
    section_id: str,
    format: str,
    section_def: SectionDef | None = None,
) -> tuple[int, int]:
    """Find the byte range of a section including its markers."""
    start_marker, end_marker = resolve_markers(section_id, format, section_def)
    si = content.find(start_marker)
    ei = content.find(end_marker)
    if si == -1 or ei == -1:
        raise ValueError(f"Section markers not found: {section_id}")
    return si, ei + len(end_marker)


def find_section_def(
    sections: list[SectionDef] | None,
    section_id: str,
) -> SectionDef | None:
    """Look up a SectionDef by ID from an optional list."""
    if sections is None:
        return None
    for s in sections:
        if s.id == section_id:
            return s
    return None


def extract_sections_regex(format: str) -> str | None:
    """Return a regex pattern for discovering section markers in the given format.

    Returns None for unsupported formats.
    """
    style = marker_style_for_format(format)
    if style == MarkerStyle.HTML_COMMENT:
        return r"<!-- section:(\S+) -->"
    if style == MarkerStyle.C_STYLE_REGION:
        return r"// #region (\S+)"
    if style == MarkerStyle.HASH_REGION:
        return r"# region (\S+)"
    return None
