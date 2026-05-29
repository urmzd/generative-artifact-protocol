Create a single self-contained TypeScript data module for a product catalog. Output one `.ts` file with no imports and no external dependencies.

The module must contain:
- An exported `interface CatalogRecord` with EXACTLY these fields, in this order:
  `id: string`, `name: string`, `sku: string`, `price: number`, `category: string`, `stock: number`.
- An exported `type Category = "electronics" | "apparel" | "home" | "outdoors" | "toys"`.

Then define EXACTLY 100 catalog records, organized into 5 logical pages of EXACTLY 20 records each.
Declare each page as its own exported const array of `CatalogRecord[]`:
`pageOne`, `pageTwo`, `pageThree`, `pageFour`, `pageFive`. Each page holds 20 records.

Numbering and stable, addressable structure (follow EXACTLY):
- Records are numbered 1 through 100 in reading order: pageOne holds records 1–20, pageTwo holds 21–40, pageThree holds 41–60, pageFour holds 61–80, pageFive holds 81–100.
- For record N, set `id: "rec-NNN"` zero-padded to 3 digits (e.g. `rec-001`, `rec-067`, `rec-100`).
- For record N, set `sku: "SKU-NNN"` zero-padded to 3 digits (e.g. `SKU-001`, `SKU-067`).
- `name` is a short product name, distinct per record (e.g. `"Product 001"` style or a plausible product name — but each name must be unique).
- `price` is a number with two decimals (e.g. `19.99`).
- `category` is one of the `Category` union literal values.
- `stock` is a non-negative integer.

Finally, export `export const catalog: CatalogRecord[] = [...pageOne, ...pageTwo, ...pageThree, ...pageFour, ...pageFive];` so the full 100-record list is addressable as one array.

The file must be valid TypeScript that compiles with no errors. Keep one record literal per logical block so individual records can be edited later.
