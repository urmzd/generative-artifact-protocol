"""Format-aware section marker utilities — mirrors src/markers.rs."""

from __future__ import annotations


def marker_style(fmt: str) -> str:
    """Return 'html', 'c', 'hash', or 'unsupported'."""
    if fmt in ("text/html", "text/markdown", "image/svg+xml"):
        return "html"
    if fmt in (
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
    ):
        return "c"
    if fmt in (
        "text/x-python",
        "text/x-ruby",
        "application/x-sh",
        "text/x-shellscript",
        "application/x-toml",
        "text/x-yaml",
        "application/yaml",
        "text/x-perl",
        "text/x-r",
    ):
        return "hash"
    if fmt == "application/json":
        return "unsupported"
    if fmt.endswith("+xml") or fmt.startswith("text/"):
        return "html"
    return "unsupported"


def markers_for(section_id: str, fmt: str) -> tuple[str, str] | None:
    style = marker_style(fmt)
    if style == "html":
        return f"<!-- section:{section_id} -->", f"<!-- /section:{section_id} -->"
    if style == "c":
        return f"// #region {section_id}", f"// #endregion {section_id}"
    if style == "hash":
        return f"# region {section_id}", f"# endregion {section_id}"
    return None


def marker_example(fmt: str) -> str:
    style = marker_style(fmt)
    if style == "html":
        return "<!-- section:ID --> ... <!-- /section:ID -->"
    if style == "c":
        return "// #region ID ... // #endregion ID"
    if style == "hash":
        return "# region ID ... # endregion ID"
    return ""


def extract_section_content(content: str, section_id: str, fmt: str) -> str | None:
    pair = markers_for(section_id, fmt)
    if not pair:
        return None
    start, end = pair
    si = content.find(start)
    ei = content.find(end)
    if si == -1 or ei == -1:
        return None
    return content[si + len(start) : ei]
