import json
import sqlite3
import time
import logging
from dataclasses import dataclass
from typing import Dict, Any, List
import requests
from bs4 import BeautifulSoup
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

<aap:target id="config-block">
@dataclass
class ScraperConfig:
    base_url: str = "https://example.com"
    rate_limit: float = 1.5
    retry_total: int = 3
    retry_backoff: float = 2.0
    output_jsonl: str = "products.jsonl"
    db_path: str = "products.db"
    user_agents: List[str] = None
</aap:target>

<aap:target id="fetcher-block">
class ProductFetcher:
    def __init__(self, config: ScraperConfig):
        self.config = config
        self.session = requests.Session()
        retries = Retry(total=config.retry_total, backoff_factor=config.retry_backoff)
        self.session.mount("https://", HTTPAdapter(max_retries=retries))
        self.session.headers.update({"User-Agent": "Mozilla/5.0"})

    def fetch(self, url: str) -> str:
        time.sleep(self.config.rate_limit)
        response = self.session.get(url, timeout=10)
        response.raise_for_status()
        return response.text
</aap:target>

<aap:target id="parser-block">
class ProductParser:
    @staticmethod
    def parse_product(html: str) -> Dict[str, Any]:
        soup = BeautifulSoup(html, 'html.parser')
        return {
            "name": soup.select_one(".product-title").text.strip(),
            "price": soup.select_one(".price").text.strip(),
            "rating": soup.select_one(".rating").get("data-value"),
            "review_count": soup.select_one(".reviews").text.strip(),
            "availability": soup.select_one(".stock").text.strip(),
            "image_url": soup.select_one("img.main-image").get("src")
        }
</aap:target>

<aap:target id="storage-block">
class Storage:
    def __init__(self, config: ScraperConfig):
        self.config = config
        self._init_db()

    def _init_db(self):
        with sqlite3.connect(self.config.db_path) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS products (
                    id INTEGER PRIMARY KEY,
                    name TEXT, price TEXT, rating TEXT, 
                    reviews TEXT, stock TEXT, img TEXT
                )
            """)

    def save(self, data: Dict[str, Any]):
        with open(self.config.output_jsonl, "a") as f:
            f.write(json.dumps(data) + "\n")
        
        with sqlite3.connect(self.config.db_path) as conn:
            conn.execute("INSERT INTO products (name, price, rating, reviews, stock, img) VALUES (?,?,?,?,?,?)",
                         (data['name'], data['price'], data['rating'], data['review_count'], data['availability'], data['image_url']))
</aap:target>