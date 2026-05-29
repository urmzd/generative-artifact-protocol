Produce a large, self-contained PostgreSQL seed script (.sql) for an e-commerce database.

First emit the schema, then the seed data organized into clearly labeled batches.

SCHEMA (emit these CREATE TABLE statements first, in this order):
- products(id INTEGER PRIMARY KEY, sku TEXT, name TEXT, category TEXT, price NUMERIC(10,2), stock INTEGER)
- users(id INTEGER PRIMARY KEY, email TEXT, full_name TEXT, country TEXT, created_at DATE)
- orders(id INTEGER PRIMARY KEY, user_id INTEGER, product_id INTEGER, quantity INTEGER, total NUMERIC(10,2), status TEXT, ordered_at DATE)

SEED DATA — emit EXACTLY 120 data rows total, organized into 6 batches (the batches are the "pages"). Use ONE single-row statement per row: every row MUST be its own `INSERT INTO <table> (...) VALUES (...);` statement on its own line. Do NOT combine multiple rows into one multi-row VALUES list. Precede each batch with a SQL comment line naming it, exactly like `-- Batch 1: products 1-20`.

- Batch 1: products 1-20    (20 rows into products, ids 1 through 20)
- Batch 2: products 21-40   (20 rows into products, ids 21 through 40)
- Batch 3: users 1-20       (20 rows into users, ids 1 through 20)
- Batch 4: users 21-40      (20 rows into users, ids 21 through 40)
- Batch 5: orders 1-20      (20 rows into orders, ids 1 through 20)
- Batch 6: orders 21-40     (20 rows into orders, ids 21 through 40)

Requirements:
- Use sequential integer ids with no gaps within each table (products 1-40, users 1-40, orders 1-40).
- products.sku must be unique and follow the pattern `SKU-0001` (zero-padded to 4 digits matching the id).
- products.category is one of: Electronics, Apparel, Home, Sports, Books.
- products.price is a realistic decimal; products.stock is an integer 0-500.
- users.email is unique; users.country is a 2-letter code (US, CA, GB, DE, FR, AU).
- orders.status starts as 'pending' for every order. orders.user_id references a users id 1-40; orders.product_id references a products id 1-40.
- Realistic but varied values. Valid PostgreSQL syntax. No external extensions.