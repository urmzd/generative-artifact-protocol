use std::time::Duration;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum WeatherError {
    #[error("HTTP request failed")]
    HttpError(#[from] reqwest::Error),
    #[error("Request timed out")]
    Timeout,
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

impl WeatherClient {
    pub async fn retry_with_backoff<F, Fut, T>(&self, mut operation: F) -> Result<T, WeatherError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, WeatherError>>,
    {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = Duration::from_millis(500);

        loop {
            match operation().await {
                Ok(val) => return Ok(val),
                Err(e) if attempts < max_attempts => {
                    match e {
                        WeatherError::HttpError(_) | WeatherError::Timeout => {
                            attempts += 1;
                            sleep(delay).await;
                            delay *= 2;
                        }
                        _ => return Err(e),
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn execute_request(&self, url: String) -> Result<reqwest::Response, WeatherError> {
        self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WeatherError::Timeout
                } else {
                    WeatherError::HttpError(e)
                }
            })
    }
}