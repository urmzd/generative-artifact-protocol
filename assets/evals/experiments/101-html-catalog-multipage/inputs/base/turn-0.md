Create a large, self-contained HTML paginated product catalog with inline CSS only.

Structure:
- Top header with the catalog title "Acme Parts Catalog" and a search input.
- Generate EXACTLY 120 products, organized into 5 pages of EXACTLY 24 products each (page 1 = products 1-24, page 2 = products 25-48, page 3 = products 49-72, page 4 = products 73-96, page 5 = products 97-120).
- Each page is a <section> with a page header reading "Page N of 5" and a grid of product cards.
- Each product is a card containing exactly these fields, each clearly labeled: id (P0001 .. P0120, zero-padded), name, sku (SKU-XXXXX), price (formatted like $12.99), category (one of: Engine, Brakes, Electrical, Suspension, Filters), stock (an integer quantity).
- A pagination navigation bar at the bottom with numbered links 1 2 3 4 5.

Requirements:
- Every product card must be a distinct, addressable element. Use the product id as a stable handle.
- Product ids must be strictly sequential P0001 through P0120 with no gaps or duplicates.
- All CSS inline in a single <style> block. No CDN, no external resources, no JavaScript. Modern, clean design.