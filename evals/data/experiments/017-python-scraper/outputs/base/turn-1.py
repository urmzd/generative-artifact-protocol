@dataclass
class Config:
    base_url: str = "https://example-ecommerce.com"
    # 2 requests per second = 0.5 seconds delay
    rate_limit: float = 0.5 
    proxy_url: Optional[str] = None
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

# Inside Fetcher class
    def fetch(self, url: str) -> Optional[str]:
        time.sleep(self.config.rate_limit)
        headers = {"User-Agent": random.choice(self.config.user_agents)}
        proxies = {"http": self.config.proxy_url, "https": self.config.proxy_url} if self.config.proxy_url else None
        
        try:
            response = self.session.get(url, headers=headers, proxies=proxies, timeout=10)
            response.raise_for_status()
            return response.text
        except Exception as e:
            logger.error(f"Failed to fetch {url}: {e}")
            return None