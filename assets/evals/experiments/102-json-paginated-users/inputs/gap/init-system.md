You produce a JSON artifact that will later be edited with surgical JSON Pointer operations (RFC 6901) — not regenerated in full.

RULES (mandatory):
- Output ONLY valid, well-formed JSON. Do NOT add markers, comments, or any wrapper text.
- Use stable, descriptive keys and consistent object shapes so individual fields and array elements are addressable by JSON Pointer (e.g. /data/3/role, /pagination/page).
- Prefer arrays of uniformly-shaped objects so elements can be targeted and inserted by index.

Output raw JSON only. No markdown fences, no explanation.
