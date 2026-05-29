Create a large, self-contained Python module named records.py that defines an inventory of product records organized by region.

Requirements:
- Define a frozen dataclass `Record` with fields: id (int), name (str), sku (str), price (float), category (str), stock (int).
- Generate EXACTLY 100 Record instances organized into 5 region groups, with EXACTLY 20 records per group. Each group is a module-level list constant:
  - REGION_NORTH   -> records with id 1 through 20
  - REGION_SOUTH   -> records with id 21 through 40
  - REGION_EAST    -> records with id 41 through 60
  - REGION_WEST    -> records with id 61 through 80
  - REGION_CENTRAL -> records with id 81 through 100
- Every record is constructed with explicit keyword arguments, one per line, e.g.:
    Record(id=1, name="...", sku="NRT-001", price=12.99, category="hardware", stock=42),
- SKU prefixes by region: NRT- (north), STH- (south), EST- (east), WST- (west), CTL- (central), followed by the zero-padded id (e.g. NRT-001, STH-021).
- Use realistic distinct names; categories drawn from: hardware, software, accessory, consumable, service. At least 15 of the 100 records MUST use category "consumable" (spread across multiple regions), so the inventory has a meaningful consumable subset.
- After the five region lists, define `ALL_RECORDS` as the concatenation of the five lists (north + south + east + west + central), preserving order.
- Define helper functions:
    def find_by_id(record_id: int) -> Record | None
    def total_stock() -> int
    def records_in_category(category: str) -> list[Record]
    def region_of(record_id: int) -> str

The module must be valid, importable Python with no external dependencies. Use stable, addressable structure: each Record(...) call on its own statement so individual records can be edited, inserted, or deleted without disturbing the others.