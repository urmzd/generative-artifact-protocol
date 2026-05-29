# Experiment: python-records-module

**Format:** text/x-python | **Size:** large | **Edits:** 5

**Multi-page design:** 100 Record dataclass instances split into 5 region lists (pages) of 20 each — REGION_NORTH (ids 1-20), REGION_SOUTH (21-40), REGION_EAST (41-60), REGION_WEST (61-80), REGION_CENTRAL (81-100) — plus helper functions and an ALL_RECORDS aggregate.

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) |
| 1 | Change price of record id=87 to 249.95 on page 5 (REGION_CENTRAL) — single field, non-first page |
| 2 | Insert new record id=101 (EST-101) after id=50 in REGION_EAST — positional insert |
| 3 | Delete record id=33 (STH-033) from REGION_SOUTH — middle deletion |
| 4 | Bulk rename category "consumable" to "supply" across every record — bulk field change |
| 5 | Add a whole new REGION_OFFSHORE page of 20 records (ids 102-121, OFS-###) — new section |
