use serde::{Deserialize, Serialize};
use reqwest::{Client as HttpClient, StatusCode};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::time::sleep;

<aap:target id="error-types">
#[derive(Error, Debug)]
pub enum WeatherError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Timeout occurred")]
    Timeout,
    #[error("Failed to parse response")]
    ParseError,
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Location not found")]
    NotFound,
}
</aap:target>

<aap:target id="data-types">
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Condition { Sunny, Cloudy, Rainy, Snowy, Stormy }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Temperature { pub celsius: f32, pub fahrenheit: f32 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WindSpeed { pub kmh: f32, pub mph: f32 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Location { pub id: String, pub name: String, pub lat: f32, pub lon: f32 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WeatherResponse { pub temp: Temperature, pub wind: WindSpeed, pub condition: Condition }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Forecast { pub days: Vec<WeatherResponse> }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirQualityResponse {
    pub aqi: i32,
    pub pm2_5: f32,
    pub pm10: f32,
    pub o3: f32,
}
</aap:target>

<aap:target id="client-struct">
pub struct WeatherClient {
    http: HttpClient,
    base_url: String,
    api_key: String,
    cache: Arc<RwLock<HashMap<String, (Instant, Vec<u8>)>>>,
}

pub struct WeatherClientBuilder {
    base_url: Option<String>,
    api_key: Option<String>,
    timeout: Duration,
}

impl WeatherClientBuilder {
    pub fn new() -> Self {
        Self { base_url: None, api_key: None, timeout: Duration::from_secs(30) }
    }
    pub fn base_url(mut self, url: &str) -> Self { self.base_url = Some(url.to_string()); self }
    pub fn api_key(mut self, key: &str) -> Self { self.api_key = Some(key.to_string()); self }
    pub fn build(self) -> WeatherClient {
        WeatherClient {
            http: HttpClient::builder().timeout(self.timeout).build().unwrap(),
            base_url: self.base_url.unwrap_or_else(|| "https://api.weather.com".to_string()),
            api_key: self.api_key.unwrap_or_default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
</aap:target>

<aap:target id="client-methods">
impl WeatherClient {
    async fn handle_status(&self, status: StatusCode) -> Result<(), WeatherError> {
        match status {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED => Err(WeatherError::InvalidApiKey),
            StatusCode::TOO_MANY_REQUESTS => Err(WeatherError::RateLimited),
            StatusCode::NOT_FOUND => Err(WeatherError::NotFound),
            _ => Err(WeatherError::ParseError),
        }
    }

    async fn get_cached<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().unwrap();
        if let Some((expiry, data)) = cache.get(key) {
            if Instant::now() < *expiry {
                return serde_json::from_slice(data).ok();
            }
        }
        None
    }

    fn set_cache<T: serde::Serialize>(&self, key: String, data: &T) {
        if let Ok(serialized) = serde_json::to_vec(data) {
            let mut cache = self.cache.write().unwrap();
            cache.insert(key, (Instant::now() + Duration::from_secs(300), serialized));
        }
    }

    async fn retry_with_backoff<F, Fut, T>(&self, f: F) -> Result<T, WeatherError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, WeatherError>>,
    {
        let mut attempts = 0;
        loop {
            match f().await {
                Err(WeatherError::HttpError(e)) if e.is_timeout() => {
                    if attempts >= 3 { return Err(WeatherError::Timeout); }
                }
                Err(e @ WeatherError::HttpError(_)) => return Err(e),
                res => return res,
            }
            attempts += 1;
            sleep(Duration::from_millis(200 * 2u64.pow(attempts))).await;
        }
    }

    pub async fn current_weather(&self, lat: f32, lon: f32) -> Result<WeatherResponse, WeatherError> {
        let key = format!("current:{}:{}", lat, lon);
        if let Some(cached) = self.get_cached::<WeatherResponse>(&key).await { return Ok(cached); }
        
        let res = self.retry_with_backoff(|| async {
            let resp = self.http.get(format!("{}/current", self.base_url))
                .query(&[("lat", lat), ("lon", lon), ("key", &self.api_key)])
                .send().await?;
            self.handle_status(resp.status()).await?;
            resp.json().await.map_err(|_| WeatherError::ParseError)
        }).await?;
        
        self.set_cache(key, &res);
        Ok(res)
    }

    pub async fn forecast(&self, lat: f32, lon: f32) -> Result<Forecast, WeatherError> {
        let key = format!("forecast:{}:{}", lat, lon);
        if let Some(cached) = self.get_cached::<Forecast>(&key).await { return Ok(cached); }
        
        let res = self.retry_with_backoff(|| async {
            let resp = self.http.get(format!("{}/forecast", self.base_url))
                .query(&[("lat", lat), ("lon", lon), ("days", 5), ("key", &self.api_key)])
                .send().await?;
            self.handle_status(resp.status()).await?;
            resp.json().await.map_err(|_| WeatherError::ParseError)
        }).await?;
        
        self.set_cache(key, &res);
        Ok(res)
    }

    pub async fn search_location(&self, query: &str) -> Result<Vec<Location>, WeatherError> {
        let key = format!("search:{}", query);
        if let Some(cached) = self.get_cached::<Vec<Location>>(&key).await { return Ok(cached); }

        let res = self.retry_with_backoff(|| async {
            let resp = self.http.get(format!("{}/search", self.base_url))
                .query(&[("q", query), ("key", &self.api_key)])
                .send().await?;
            self.handle_status(resp.status()).await?;
            resp.json().await.map_err(|_| WeatherError::ParseError)
        }).await?;
        
        self.set_cache(key, &res);
        Ok(res)
    }

    pub async fn air_quality(&self, location: &Location) -> Result<AirQualityResponse, WeatherError> {
        let key = format!("aqi:{}", location.id);
        if let Some(cached) = self.get_cached::<AirQualityResponse>(&key).await { return Ok(cached); }

        let res = self.retry_with_backoff(|| async {
            let resp = self.http.get(format!("{}/air-quality", self.base_url))
                .query(&[("lat", location.lat), ("lon", location.lon), ("key", &self.api_key)])
                .send().await?;
            self.handle_status(resp.status()).await?;
            resp.json().await.map_err(|_| WeatherError::ParseError)
        }).await?;
        
        self.set_cache(key, &res);
        Ok(res)
    }
}
</aap:target>
