@staticmethod
def parse_product(html: str) -> Dict[str, Any]:
    soup = BeautifulSoup(html, 'html.parser')
    
    # Extract raw prices
    price_elem = soup.select_one(".price")
    sale_elem = soup.select_one(".discount-price")
    
    original_price = float(price_elem.text.replace('$', '').strip()) if price_elem else 0.0
    discount_price = float(sale_elem.text.replace('$', '').strip()) if sale_elem else original_price
    
    # Calculate discount percentage
    discount_percent = 0.0
    if original_price > 0 and discount_price < original_price:
        discount_percent = ((original_price - discount_price) / original_price) * 100

    return {
        "name": soup.select_one(".product-title").text.strip() if soup.select_one(".product-title") else None,
        "price": original_price,
        "discount_price": discount_price,
        "discount_percent": round(discount_percent, 2),
        "rating": float(soup.select_one(".rating").get('data-score', 0)) if soup.select_one(".rating") else 0.0,
        "reviews": int(soup.select_one(".review-count").text.split()[0]) if soup.select_one(".review-count") else 0,
        "availability": soup.select_one(".stock").text.strip() if soup.select_one(".stock") else "Unknown",
        "image_url": soup.select_one(".product-img")['src'] if soup.select_one(".product-img") else None
    }

# Update storage layer to include new fields
def _init_db(self):
    with sqlite3.connect(self.config.db_path) as conn:
        conn.execute("""
            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT, price REAL, discount_price REAL, discount_percent REAL,
                rating REAL, reviews INTEGER, availability TEXT, image_url TEXT
            )
        """)

def save(self, data: Dict[str, Any]):
    with open(self.config.output_json, 'a', encoding='utf-8') as f:
        f.write(json.dumps(data) + '\n')
    
    with sqlite3.connect(self.config.db_path) as conn:
        conn.execute("""
            INSERT INTO products (name, price, discount_price, discount_percent, rating, reviews, availability, image_url)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (data['name'], data['price'], data['discount_price'], data['discount_percent'], 
              data['rating'], data['reviews'], data['availability'], data['image_url']))