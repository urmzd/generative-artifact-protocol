# Experiment: sql-seed-multi-batch

**Format:** application/sql | **Size:** large | **Edits:** 5

**Multi-page design:** 120 seed rows across 6 batches (20 each) — 40 products, 40 users, 40 orders; each batch is a "page". One single-row INSERT per row so rows are individually addressable.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 96 | 24 |
| GAP init system | 1217 | 304 |
| GAP maintain system | 267 | 67 |
| **Protocol overhead** | | **~347 tokens** |

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) |
| 1 | Single field on a non-first page: set product 27 (Batch 2) price 1299.99, stock 7 |
| 2 | Insert: append product 41 (SKU-0041, Aurora Wireless Headphones) to end of Batch 2 |
| 3 | Delete: remove order id 15 from the middle of Batch 5 |
| 4 | Bulk: change status of every remaining order from 'pending' to 'shipped' (Batches 5 & 6) |
| 5 | Add a page: new reviews table + Batch 7 with 20 review rows |

## Row-count invariants (checked each turn)
| After turn | products | users | orders | reviews | total INSERTs |
|---|---|---|---|---|---|
| 0 | 40 | 40 | 40 | 0 | 120 |
| 1 | 40 | 40 | 40 | 0 | 120 |
| 2 | 41 | 40 | 40 | 0 | 121 |
| 3 | 41 | 40 | 39 | 0 | 120 |
| 4 | 41 | 40 | 39 | 0 | 120 |
| 5 | 41 | 40 | 39 | 20 | 140 |
