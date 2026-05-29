# Experiment: xml-rss-feed-multipage

**Format:** application/xml | **Size:** large | **Edits:** 5

**Multi-page design:** 80 RSS `<item>` entries (each with title, link, guid, pubDate, category, description) organized conceptually into 4 batches of 20 (batch 1 = items 1-20, batch 2 = 21-40, batch 3 = 41-60, batch 4 = 61-80), plus channel metadata. Edits grow the feed to 81, then back to 80, then to 100 (a new 5th batch).

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) Generate an 80-item RSS 2.0 feed in 4 batches of 20 with channel metadata |
| 1 | Single field, non-first page: change the title of item 67 (batch 4) to "Quantum Chips Enter Mass Production" |
| 2 | Insert: add a new item (guid dailybyte-0081) at the start of batch 2, before item 21; total becomes 81 |
| 3 | Delete: remove item 50 (guid dailybyte-0050) from batch 3; total back to 80 |
| 4 | Bulk field change: set `<category>` to "Technology" for every item across all batches |
| 5 | Add a whole new batch: append 20 new items (guids dailybyte-0101..0120); total becomes 100 |
