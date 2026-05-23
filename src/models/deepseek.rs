use reqwest::blocking::Client;

use super::ModelInfo;
use super::ModelProvider;

pub struct DeepSeekProvider;

impl ModelProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "DeepSeek"
    }

    fn default_api_url(&self) -> &str {
        "https://api.deepseek.com/v1"
    }

    fn fetch_models(&self, api_key: &str, api_url: &str) -> Result<Vec<ModelInfo>, String> {
        let client = Client::builder()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let url = format!("{}/models", api_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("HTTP request to {} failed: {}", url, e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(format!(
                "API returned error status ({}): {}",
                status, error_text
            ));
        }

        let body_text = response
            .text()
            .map_err(|e| format!("Failed to read response text: {}", e))?;

        #[derive(serde::Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelInfo>,
        }

        let parsed: ModelsResponse = serde_json::from_str(&body_text).map_err(|e| {
            format!(
                "Failed to parse models JSON response: {}. Response was: {}",
                e, body_text
            )
        })?;

        Ok(parsed.data)
    }
}
