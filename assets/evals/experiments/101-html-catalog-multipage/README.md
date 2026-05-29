# Experiment: html-catalog-multipage

**Format:** text/html | **Size:** large | **Edits:** 5

**Multi-page design:** 120 products (ids P0001-P0120) across 5 pages of 24 product cards each; each page is a `<section>` with a "Page N of 5" header, and a pagination nav (1 2 3 4 5) at the bottom.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 90 | 22 |
| GAP init system | 1217 | 304 |
| GAP maintain system | 267 | 67 |
| **Protocol overhead** | | **~349 tokens** |

## Turns

| Turn | Edit |
|---|---|
| 0 | (creation) — 120 products, 5 pages of 24 |
| 1 | Single field, non-first page: change price of P0087 (page 4) to $349.95 |
| 2 | Insert: add P0121 on page 2 after P0030 (catalog -> 121) |
| 3 | Delete: remove P0055 from page 3 (catalog -> 120) |
| 4 | Bulk: change every "Filters" category to "Air Systems" across all pages |
| 5 | Add page: new page 6 with 12 products P0122-P0133 (catalog -> 132) |

## Multi-page targeting patterns exercised

1. **Single field, non-first page** (turn 1): isolate one value on page 4 without touching the other 119 products.
2. **Insert at position** (turn 2): add one item mid-document on page 2; count must rise to exactly 121.
3. **Delete from middle** (turn 3): remove one item from page 3; count must fall to exactly 120; neighbors P0054/P0056 must survive un-renumbered.
4. **Bulk field change** (turn 4): the hardest GAP case — many ops or a full pass; "Filters" must be entirely absent afterward.
5. **Add whole page** (turn 5): a new section of 12 items plus header/nav updates; count must reach exactly 132.

## Correctness oracle

Each `checks/turn-N.json` encodes the intended outcome: the targeted new value is present, removed/old values are absent where applicable, and `regex_count` over `P0\d{3}` pins the EXACT total product count after each cumulative edit (120 -> 120 -> 121 -> 120 -> 120 -> 132). The count assertion is what catches a flow that "succeeds" on the targeted change but silently drops the other items.
