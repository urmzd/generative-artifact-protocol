# Experiment: json-paginated-users

**Format:** application/json | **Size:** large | **Edits:** 5

**Multi-page design:** 100 user objects in `data`, modeled as 5 pages of 20 (page 1 = ids 1-20, page 2 = ids 21-40, page 3 = ids 41-60, page 4 = ids 61-80, page 5 = ids 81-100); `pagination` holds page/per_page/total/total_pages.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 97 | 24 |
| GAP init system | 566 | 142 |
| GAP maintain system | 952 | 238 |
| **Protocol overhead** | | **~356 tokens** |

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) |
| 1 | Change email of user id 67 (page 4, index 66) — single field, single item, non-first page |
| 2 | Insert new user (id 1001) after id 45 on page 3; total -> 101 |
| 3 | Delete user id 53 from the middle (page 3); total -> 100 |
| 4 | Bulk set active=false for ALL 100 users (GAP's hardest case) |
| 5 | Append a whole new page 6 (ids 2001-2020); total -> 120, total_pages -> 6 |
