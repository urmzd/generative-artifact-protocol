import logging
import time
import json
import sqlite3
import random
from dataclasses import dataclass
from typing import Optional, Dict, Any
from pathlib import Path

import requests
from bs4 import BeautifulSoup
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class Config:
    base_url: str = "https://example-ecommerce.com"
    rate_limit: float = 2.0
    max_retries: int = 3
    backoff_factor: float = 1.0
    output_json: str = "products.jsonl"
    db_path: str = "products.db"
    user_agents: list = None

    def __post_init__(self):
        if self.user_agents is None:
            self.user_agents = [
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.107 Safari/537.36"
            ]

class Storage:
    def __init__(self, config: Config):
        self.config = config
        self._init_db()

    def _init_db(self):
        with sqlite3.connect(self.config.db_path) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS products (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT, price REAL, rating REAL, 
                    reviews INTEGER, availability TEXT, image_url TEXT
                )
            """)

    def save(self, data: Dict[str, Any]):
        with open(self.config.output_json, 'a', encoding='utf-8') as f:
            f.write(json.dumps(data) + '\n')
        
        with sqlite3.connect(self.config.db_path) as conn:
            conn.execute("""
                INSERT INTO products (name, price, rating, reviews, availability, image_url)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (data['name'], data['price'], data['rating'], data['reviews'], data['availability'], data['image_url']))

class Fetcher:
    def __init__(self, config: Config):
        self.config = config
        self.session = requests.Session()
        retries = Retry(total=config.max_retries, backoff_factor=config.backoff_factor)
        self.session.mount("https://", HTTPAdapter(max_retries=retries))

    def fetch(self, url: str) -> Optional[str]:
        time.sleep(self.config.rate_limit)
        headers = {"User-Agent": random.choice(self.config.user_agents)}
        try:
            response = self.session.get(url, headers=headers, timeout=10)
            response.raise_for_status()
            return response.text
        except Exception as e:
            logger.error(f"Failed to fetch {url}: {e}")
            return None

class Parser:
    @staticmethod
    def parse_product(html: str) -> Dict[str, Any]:
        soup = BeautifulSoup(html, 'html.parser')
        # Selectors are placeholders for demonstration
        return {
            "name": soup.select_one(".product-title").text.strip() if soup.select_one(".product-title") else None,
            "price": float(soup.select_one(".price").text.replace('$', '').strip()) if soup.select_one(".price") else 0.0,
            "rating": float(soup.select_one(".rating").get('data-score', 0)) if soup.select_one(".rating") else 0.0,
            "reviews": int(soup.select_one(".review-count").text.split()[0]) if soup.select_one(".review-count") else 0,
            "availability": soup.select_one(".stock").text.strip() if soup.select_one(".stock") else "Unknown",
            "image_url": soup.select_one(".product-img")['src'] if soup.select_one(".product-img") else None
        }

def main():
    config = Config()
    fetcher = Fetcher(config)
    storage = Storage(config)
    parser = Parser()

    urls = [f"{config.base_url}/product/{i}" for i in range(1, 11)]

    for url in urls:
        html = fetcher.fetch(url)
        if html:
            data = parser.parse_product(html)
            storage.save(data)
            logger.info(f"Saved product: {data.get('name')}")

if __name__ == "__main__":
    main()