use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};

struct CacheEntry<T> {
    data: T,
    expires_at: SystemTime,
}

pub struct WeatherClient {
    http_client: HttpClient,
    base_url: String,
    api_key: String,
    cache: Arc<RwLock<HashMap<String, CacheEntry<String>>>>, // Storing raw strings for generic caching
    ttl: Duration,
}

impl WeatherClient {
    fn is_expired(expires_at: SystemTime) -> bool {
        SystemTime::now() > expires_at
    }

    async fn get_cached_or_fetch<T>(
        &self,
        key: &str,
        fetch_fn: impl std::future::Future<Output = Result<T, WeatherError>>,
    ) -> Result<T, WeatherError> 
    where T: serde::de::DeserializeOwned + serde::Serialize + Clone
    {
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(key) {
                if !Self::is_expired(entry.expires_at) {
                    if let Ok(data) = serde_json::from_str::<T>(&entry.data) {
                        return Ok(data);
                    }
                }
            }
        }

        let data = fetch_fn.await?;
        
        if let Ok(serialized) = serde_json::to_string(&data) {
            let mut cache = self.cache.write().unwrap();
            cache.insert(key.to_string(), CacheEntry {
                data: serialized,
                expires_at: SystemTime::now() + self.ttl,
            });
        }
        
        Ok(data)
    }
}

// Updated Builder
impl WeatherClientBuilder {
    pub fn build(self) -> WeatherClient {
        let http_client = HttpClient::builder()
            .timeout(self.timeout)
            .build()
            .unwrap();

        WeatherClient {
            http_client,
            base_url: self.base_url,
            api_key: self.api_key,
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(300), // 5 minutes
        }
    }
}