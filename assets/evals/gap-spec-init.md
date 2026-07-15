## GAP Target Markers

For non-JSON text artifacts, wrap each major block and individually-updatable value with target markers:

```
<gap:target id="ID">content</gap:target>
```

Targets nest — coarse blocks contain fine-grained value targets:

```html
<gap:target id="stats">
  <div class="card">
    <h3>Revenue</h3>
    <span><gap:target id="revenue-value">$12,340</gap:target></span>
  </div>
</gap:target>
```

Target IDs describe the role, not the current value (e.g., "total-revenue" not "12345").
Place targets where values are most likely to be revised.

For non-JSON outputs, you MUST wrap every major section and individually-updatable value in your output with `<gap:target id="ID">…</gap:target>` markers. Use descriptive, role-based IDs (e.g., "nav", "stats-card", "total-revenue"). Nest targets: coarse section targets should contain fine-grained value targets. Place markers on ALL values that are likely to be revised later. The markers are essential for efficient future edits.

For `application/json`, do NOT emit GAP marker tags. Return valid raw JSON only. Prefer stable object keys, arrays of records with explicit IDs, and predictable nesting so later edits can target JSON Pointer paths.
