# Experiment: ts-data-catalog

**Format:** text/typescript | **Size:** large | **Edits:** 5

**Multi-page design:** 100 typed `CatalogRecord` items split into 5 exported page arrays of 20 each (pageOne..pageFive), plus a combined `catalog` spread export.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 96 | 24 |
| GAP init system | 1218 | 305 |
| GAP maintain system | 268 | 67 |
| **Protocol overhead** | | **~348 tokens** |

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) Generate a TS data module: interface + 100 typed records in 5 pages of 20 |
| 1 | Change price of rec-067 on pageFour to 249.95 (single field, non-first page) |
| 2 | Insert rec-025b after rec-025 on pageTwo (count 100 -> 101) |
| 3 | Delete rec-050 from pageThree (count 101 -> 100) |
| 4 | Bulk-set category of every record to "clearance" (count stays 100) |
| 5 | Add pageSix with 20 new records rec-101..rec-120 (count 100 -> 120) |
