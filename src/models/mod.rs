use serde::{Deserialize, Serialize};

pub mod deepseek;

/// Information about an available model from a provider.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub owned_by: String,
}

/// Trait that every model provider (DeepSeek, OpenAI, etc.) must implement.
pub trait ModelProvider {
    /// Human-readable provider name (e.g. "DeepSeek").
    fn name(&self) -> &str;

    /// Default API base URL for this provider.
    fn default_api_url(&self) -> &str;

    /// Fetch the list of available models from the provider's API.
    /// Requires a valid API key for authorization.
    fn fetch_models(&self, api_key: &str, api_url: &str) -> Result<Vec<ModelInfo>, String>;

    /// Get the chat completions endpoint path relative to the API base URL.
    fn completions_path(&self) -> &str {
        "/chat/completions"
    }
}

/// Registry of all supported model providers.
pub fn get_provider(name: &str) -> Option<Box<dyn ModelProvider>> {
    match name.to_lowercase().as_str() {
        "deepseek" => Some(Box::new(deepseek::DeepSeekProvider)),
        _ => None,
    }
}

/// List all registered provider names.
pub fn list_providers() -> Vec<&'static str> {
    vec!["deepseek"]
}

/// Guess which provider handles the given model name.
/// Scans registered provider names against the model string (case-insensitive).
pub fn infer_provider_from_model(model: &str) -> &str {
    let lower = model.to_lowercase();
    for name in list_providers() {
        if lower.contains(name) {
            return name;
        }
    }
    "deepseek"
}

/// Resolve the effective API base URL:
/// - If `configured_url` is non-empty, the user has set a custom URL; use it.
/// - Otherwise, look up the provider's default URL (inferred from the model name).
pub fn resolve_api_url(configured_url: &str, model: &str) -> String {
    if !configured_url.is_empty() {
        return configured_url.to_string();
    }
    let provider_name = infer_provider_from_model(model);
    get_provider(provider_name)
        .map(|p| p.default_api_url().to_string())
        .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string())
}
