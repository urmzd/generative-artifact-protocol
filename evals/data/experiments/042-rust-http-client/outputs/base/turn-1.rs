#[derive(Debug, Deserialize)]
pub struct AirQualityResponse {
    pub aqi: i32,
    pub pm2_5: f32,
    pub pm10: f32,
    pub o3: f32,
}

impl WeatherClient {
    pub async fn air_quality(&self, location_id: &str) -> Result<AirQualityResponse, WeatherError> {
        let url = format!(
            "{}/air_quality?id={}&appid={}",
            self.base_url, location_id, self.api_key
        );
        let resp = self.http_client.get(url).send().await?;
        self.handle_response(resp).await
    }
}