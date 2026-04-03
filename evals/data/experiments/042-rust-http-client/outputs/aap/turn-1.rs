use serde::{Deserialize, Serialize};
use reqwest::{Client as HttpClient, StatusCode};
use std::time::Duration;
use thiserror::Error;

<aap:target id="error-types">
#[derive(Error, Debug)]
pub enum WeatherError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
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
#[derive(Debug, Deserialize, Serialize)]
pub enum Condition { Sunny, Cloudy, Rainy, Snowy, Stormy }

#[derive(Debug, Deserialize)]
pub struct Temperature { pub celsius: f32, pub fahrenheit: f32 }

#[derive(Debug, Deserialize)]
pub struct WindSpeed { pub kmh: f32, pub mph: f32 }

#[derive(Debug, Deserialize)]
pub struct Location { pub id: String, pub name: String, pub lat: f32, pub lon: f32 }

#[derive(Debug, Deserialize)]
pub struct WeatherResponse { pub temp: Temperature, pub wind: WindSpeed, pub condition: Condition }

#[derive(Debug, Deserialize)]
pub struct Forecast { pub days: Vec<WeatherResponse> }

#[derive(Debug, Deserialize)]
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

    pub async fn current_weather(&self, lat: f32, lon: f32) -> Result<WeatherResponse, WeatherError> {
        let resp = self.http.get(format!("{}/current", self.base_url))
            .query(&[("lat", lat), ("lon", lon), ("key", &self.api_key)])
            .send().await?;
        self.handle_status(resp.status()).await?;
        resp.json().await.map_err(|_| WeatherError::ParseError)
    }

    pub async fn forecast(&self, lat: f32, lon: f32) -> Result<Forecast, WeatherError> {
        let resp = self.http.get(format!("{}/forecast", self.base_url))
            .query(&[("lat", lat), ("lon", lon), ("days", 5), ("key", &self.api_key)])
            .send().await?;
        self.handle_status(resp.status()).await?;
        resp.json().await.map_err(|_| WeatherError::ParseError)
    }

    pub async fn search_location(&self, query: &str) -> Result<Vec<Location>, WeatherError> {
        let resp = self.http.get(format!("{}/search", self.base_url))
            .query(&[("q", query), ("key", &self.api_key)])
            .send().await?;
        self.handle_status(resp.status()).await?;
        resp.json().await.map_err(|_| WeatherError::ParseError)
    }

    pub async fn air_quality(&self, location: &Location) -> Result<AirQualityResponse, WeatherError> {
        let resp = self.http.get(format!("{}/air-quality", self.base_url))
            .query(&[("lat", location.lat), ("lon", location.lon), ("key", &self.api_key)])
            .send().await?;
        self.handle_status(resp.status()).await?;
        resp.json().await.map_err(|_| WeatherError::ParseError)
    }
}
</aap:target>
