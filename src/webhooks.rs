use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Configuration for a single webhook endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookConfig {
    /// The URL to send the POST request to.
    pub url: String,
    /// Which events this webhook subscribes to.
    /// Valid values: "set", "rm", "lock", "unlock", "rotate", "pull", "push", "*"
    #[serde(default)]
    pub events: Vec<String>,
    /// Additional HTTP headers to include in the request.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// The payload sent to webhook endpoints.
/// NEVER includes secret values -- only the key name.
#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    /// The event that triggered the webhook (e.g. "set", "rm", "rotate").
    pub event: String,
    /// Identifier for the project.
    pub project_id: String,
    /// The environment affected, if applicable.
    pub environment: Option<String>,
    /// The variable name affected, if applicable. Never the secret value.
    pub key: Option<String>,
    /// ISO 8601 timestamp of when the event occurred.
    pub timestamp: String,
    /// The user who triggered the event.
    pub user: String,
}

/// Top-level structure for parsing the webhooks section of .envsafe.yaml.
#[derive(Debug, Deserialize)]
struct EnvsafeYaml {
    #[serde(default)]
    webhooks: Vec<WebhookConfig>,
}

/// Load webhook configurations from `.envsafe.yaml` in the given project root.
///
/// Returns an empty vec if the file does not exist or has no webhooks section.
pub fn load_webhooks(project_root: &Path) -> Vec<WebhookConfig> {
    let config_path = project_root.join(".envsafe.yaml");

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    match serde_yaml::from_str::<EnvsafeYaml>(&content) {
        Ok(yaml) => yaml.webhooks,
        Err(e) => {
            eprintln!(
                "Warning: failed to parse webhooks from {}: {}",
                config_path.display(),
                e
            );
            Vec::new()
        }
    }
}

/// Send a webhook notification to all matching webhook endpoints.
///
/// A webhook matches if its `events` list contains the given event name or "*".
/// An empty events list is treated as matching all events.
///
/// This function never fails the caller -- webhook errors are logged as warnings
/// to stderr but do not propagate.
pub fn notify(webhooks: &[WebhookConfig], event: &str, payload: &WebhookPayload) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: failed to create HTTP client for webhooks: {e}");
            return;
        }
    };

    for webhook in webhooks {
        if !should_fire(webhook, event) {
            continue;
        }

        if let Err(e) = send_webhook(&client, webhook, payload) {
            eprintln!("Warning: webhook to {} failed: {}", webhook.url, e);
        }
    }
}

/// Check whether a webhook config should fire for the given event.
fn should_fire(webhook: &WebhookConfig, event: &str) -> bool {
    if webhook.events.is_empty() {
        return true;
    }
    webhook.events.iter().any(|e| e == "*" || e == event)
}

/// Send a single webhook POST request.
fn send_webhook(
    client: &reqwest::blocking::Client,
    webhook: &WebhookConfig,
    payload: &WebhookPayload,
) -> Result<()> {
    let mut request = client
        .post(&webhook.url)
        .header("Content-Type", "application/json");

    for (key, value) in &webhook.headers {
        request = request.header(key.as_str(), value.as_str());
    }

    let response = request.json(payload).send()?;

    if !response.status().is_success() {
        eprintln!(
            "Warning: webhook {} returned status {}",
            webhook.url,
            response.status()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_fire_wildcard() {
        let webhook = WebhookConfig {
            url: "https://example.com/hook".into(),
            events: vec!["*".into()],
            headers: HashMap::new(),
        };
        assert!(should_fire(&webhook, "set"));
        assert!(should_fire(&webhook, "rm"));
        assert!(should_fire(&webhook, "rotate"));
    }

    #[test]
    fn test_should_fire_specific_events() {
        let webhook = WebhookConfig {
            url: "https://example.com/hook".into(),
            events: vec!["set".into(), "rm".into()],
            headers: HashMap::new(),
        };
        assert!(should_fire(&webhook, "set"));
        assert!(should_fire(&webhook, "rm"));
        assert!(!should_fire(&webhook, "rotate"));
    }

    #[test]
    fn test_should_fire_empty_events() {
        let webhook = WebhookConfig {
            url: "https://example.com/hook".into(),
            events: vec![],
            headers: HashMap::new(),
        };
        // Empty events list matches everything
        assert!(should_fire(&webhook, "set"));
        assert!(should_fire(&webhook, "anything"));
    }

    #[test]
    fn test_load_webhooks_missing_file() {
        let webhooks = load_webhooks(Path::new("/nonexistent/path"));
        assert!(webhooks.is_empty());
    }

    #[test]
    fn test_payload_serialization() {
        let payload = WebhookPayload {
            event: "set".into(),
            project_id: "my-project".into(),
            environment: Some("production".into()),
            key: Some("DATABASE_URL".into()),
            timestamp: "2025-01-01T00:00:00Z".into(),
            user: "alice".into(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"event\":\"set\""));
        assert!(json.contains("\"key\":\"DATABASE_URL\""));
        // Ensure no secret values leak -- only key names
        assert!(!json.contains("password"));
    }
}
