use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WeatherError {
    #[error("HTTP request failed")]
    HttpError(#[from] reqwest::Error),
    #[error("Failed to parse response")]
    ParseError,
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Resource not found")]
    NotFound,
    #[error("Unknown error occurred")]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Condition {
    Sunny,
    Cloudy,
    Rainy,
    Snowy,
    Stormy,
}

#[derive(Debug, Deserialize)]
pub struct Temperature {
    pub current: f32,
    pub feels_like: f32,
    pub unit: String,
}

#[derive(Debug, Deserialize)]
pub struct WindSpeed {
    pub speed: f32,
    pub direction: String,
}

#[derive(Debug, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize)]
pub struct WeatherResponse {
    pub location: Location,
    pub temperature: Temperature,
    pub condition: Condition,
    pub wind: WindSpeed,
}

#[derive(Debug, Deserialize)]
pub struct Forecast {
    pub daily: Vec<WeatherResponse>,
}

pub struct WeatherClient {
    http_client: HttpClient,
    base_url: String,
    api_key: String,
}

pub struct WeatherClientBuilder {
    base_url: String,
    api_key: String,
    timeout: Duration,
}

impl WeatherClientBuilder {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> WeatherClient {
        let http_client = HttpClient::builder()
            .timeout(self.timeout)
            .build()
            .unwrap();

        WeatherClient {
            http_client,
            base_url: self.base_url,
            api_key: self.api_key,
        }
    }
}

impl WeatherClient {
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, WeatherError> {
        match response.status() {
            StatusCode::OK => response.json::<T>().await.map_err(|_| WeatherError::ParseError),
            StatusCode::UNAUTHORIZED => Err(WeatherError::InvalidApiKey),
            StatusCode::TOO_MANY_REQUESTS => Err(WeatherError::RateLimited),
            StatusCode::NOT_FOUND => Err(WeatherError::NotFound),
            _ => Err(WeatherError::Unknown),
        }
    }

    pub async fn current_weather(&self, location: &str) -> Result<WeatherResponse, WeatherError> {
        let url = format!("{}/current?q={}&appid={}", self.base_url, location, self.api_key);
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }

    pub async fn forecast(&self, location: &str) -> Result<Forecast, WeatherError> {
        let url = format!("{}/forecast?q={}&appid={}", self.base_url, location, self.api_key);
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }

    pub async fn historical(&self, location: &str, date: &str) -> Result<WeatherResponse, WeatherError> {
        let url = format!("{}/history?q={}&date={}&appid={}", self.base_url, location, date, self.api_key);
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }

    pub async fn search_location(&self, query: &str) -> Result<Vec<Location>, WeatherError> {
        let url = format!("{}/search?q={}&appid={}", self.base_url, query, self.api_key);
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }

    pub async fn alerts(&self, location: &str) -> Result<Vec<String>, WeatherError> {
        let url = format!("{}/alerts?q={}&appid={}", self.base_url, location, self.api_key);
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }
}