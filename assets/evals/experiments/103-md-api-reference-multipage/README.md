# Experiment: md-api-reference-multipage

**Format:** text/markdown | **Size:** large | **Edits:** 5

**Multi-page design:** 90 endpoint entries (`### EP-PNN`) split across 6 pages (`## ` sections) of 15 endpoints each; edits target non-first pages and a 7th page is added.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 94 | 23 |
| GAP init system | 1217 | 304 |
| GAP maintain system | 267 | 66 |
| **Protocol overhead** | | **~347 tokens** |

## Multi-page targeting patterns exercised

| Pattern | Turn |
|---|---|
| Edit a single field of a single item on a non-first page | 1 (EP-407, page 4) |
| Insert a new item at a specific position/page | 2 (EP-216, page 2) |
| Delete a specific item from the middle | 3 (EP-308, page 3) |
| Bulk change one field across all items | 4 (all 90 paths /v1/ -> /v2/) |
| Add a whole new page/section of items | 5 (page 7, EP-701..EP-715) |

## Item count ledger

| After turn | Endpoints (`### EP-`) | Pages (`## Page`) |
|---|---|---|
| 0 (creation) | 90 | 6 |
| 1 | 90 | 6 |
| 2 | 91 | 6 |
| 3 | 90 | 6 |
| 4 | 90 | 6 |
| 5 | 105 | 7 |

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) |
| 1 | Change the Path of EP-407 on Page 4 (Storage) to /v1/storage/buckets/{bucketId}/lifecycle |
| 2 | Insert new endpoint EP-216 (Export Invoices) after EP-215 on Page 2 (Billing) |
| 3 | Delete endpoint EP-308 from the middle of Page 3 (Compute) |
| 4 | Bulk-bump every endpoint Path version from /v1/ to /v2/ across all pages |
| 5 | Add a 7th page (Security) with 15 new endpoints EP-701..EP-715 |
