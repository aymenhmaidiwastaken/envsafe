use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Full .envsafe.yaml configuration file structure.
///
/// This extends the existing validation schema with project-level configuration,
/// sync settings, team definitions, and webhook support.
#[derive(Debug, Deserialize)]
pub struct EnvsafeConfig {
    /// Project metadata
    #[serde(default)]
    pub project: Option<ProjectSection>,

    /// Cloud sync configuration
    #[serde(default)]
    pub sync: Option<SyncSection>,

    /// Team member definitions with environment access
    #[serde(default)]
    pub team: Vec<TeamMember>,

    /// Webhook configuration for change notifications
    #[serde(default)]
    pub webhooks: Option<WebhooksSection>,

    /// Required environment variable schemas (backward-compatible with existing validator)
    #[serde(default)]
    pub required: Vec<VarSchema>,
}

/// Project metadata section
#[derive(Debug, Deserialize)]
pub struct ProjectSection {
    /// Human-readable project name
    #[serde(default)]
    pub name: Option<String>,

    /// Default environment to use when --env is not specified
    #[serde(default = "default_env")]
    pub default_env: String,
}

fn default_env() -> String {
    "dev".to_string()
}

/// Cloud sync provider configuration
#[derive(Debug, Deserialize)]
pub struct SyncSection {
    /// Provider name: aws-ssm, vault, 1password, gcp
    pub provider: String,

    /// Prefix for parameter paths (e.g., "/myapp/")
    #[serde(default)]
    pub prefix: Option<String>,

    /// Path for secrets (used by vault, gcp providers)
    #[serde(default)]
    pub path: Option<String>,

    /// Vault name (used by 1password)
    #[serde(default)]
    pub vault_name: Option<String>,
}

/// Team member definition
#[derive(Debug, Deserialize)]
pub struct TeamMember {
    /// Display name
    pub name: String,

    /// Email address
    pub email: String,

    /// Environments this team member has access to
    #[serde(default)]
    pub environments: Vec<String>,
}

/// Webhook configuration
#[derive(Debug, Deserialize)]
pub struct WebhooksSection {
    /// Webhook triggered when variables change
    #[serde(default)]
    pub on_change: Option<WebhookEndpoint>,
}

/// A single webhook endpoint
#[derive(Debug, Deserialize)]
pub struct WebhookEndpoint {
    /// URL to send the webhook POST request to
    pub url: String,

    /// Optional custom headers
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

/// Variable schema definition (backward-compatible with existing validator)
#[derive(Debug, Deserialize)]
pub struct VarSchema {
    /// Variable name
    pub name: String,

    /// Regex pattern the value must match
    #[serde(default)]
    pub pattern: Option<String>,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Type constraint: integer, boolean, url, string
    #[serde(default)]
    pub r#type: Option<String>,

    /// Default value if not set
    #[serde(default)]
    pub default: Option<serde_yaml::Value>,
}

/// Load and parse the .envsafe.yaml configuration file.
///
/// Searches upward from the current directory, matching the same behavior
/// as the existing validator.
pub fn load() -> Result<EnvsafeConfig> {
    let path = find_config_file()?;
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;
    let config: EnvsafeConfig =
        serde_yaml::from_str(&content).context("Invalid .envsafe.yaml format")?;
    Ok(config)
}

/// Try to load the config file, returning None if not found.
pub fn load_optional() -> Option<EnvsafeConfig> {
    load().ok()
}

/// Get the default environment from the config file, falling back to "dev".
pub fn get_default_env() -> String {
    load_optional()
        .and_then(|c| c.project)
        .map(|p| p.default_env)
        .unwrap_or_else(|| "dev".to_string())
}

/// Get the project name from the config file.
pub fn project_name() -> Option<String> {
    load_optional().and_then(|c| c.project).and_then(|p| p.name)
}

/// Get team members who have access to a specific environment.
pub fn team_for_env(env: &str) -> Vec<TeamMember> {
    load_optional()
        .map(|c| {
            c.team
                .into_iter()
                .filter(|m| m.environments.is_empty() || m.environments.iter().any(|e| e == env))
                .collect()
        })
        .unwrap_or_default()
}

/// Find the .envsafe.yaml or .envsafe.yml file by walking up directories.
fn find_config_file() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(".envsafe.yaml");
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate = current.join(".envsafe.yml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !current.pop() {
            anyhow::bail!("No .envsafe.yaml configuration file found");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
project:
  name: "my-app"
  default_env: "dev"

sync:
  provider: "aws-ssm"
  prefix: "/myapp/"

team:
  - name: "Alice"
    email: "alice@example.com"
    environments: ["dev", "staging", "prod"]
  - name: "Bob"
    email: "bob@example.com"
    environments: ["dev", "staging"]

webhooks:
  on_change:
    url: "https://hooks.slack.com/test"

required:
  - name: DATABASE_URL
    pattern: "^postgres://"
    description: "PostgreSQL connection string"
  - name: PORT
    type: integer
    default: 3000
"#;

        let config: EnvsafeConfig = serde_yaml::from_str(yaml).unwrap();

        let project = config.project.unwrap();
        assert_eq!(project.name.unwrap(), "my-app");
        assert_eq!(project.default_env, "dev");

        let sync = config.sync.unwrap();
        assert_eq!(sync.provider, "aws-ssm");
        assert_eq!(sync.prefix.unwrap(), "/myapp/");

        assert_eq!(config.team.len(), 2);
        assert_eq!(config.team[0].name, "Alice");
        assert_eq!(config.team[0].environments.len(), 3);
        assert_eq!(config.team[1].name, "Bob");

        let webhooks = config.webhooks.unwrap();
        assert_eq!(
            webhooks.on_change.unwrap().url,
            "https://hooks.slack.com/test"
        );

        assert_eq!(config.required.len(), 2);
        assert_eq!(config.required[0].name, "DATABASE_URL");
        assert!(config.required[0].pattern.is_some());
        assert_eq!(config.required[1].name, "PORT");
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
required:
  - name: API_KEY
"#;
        let config: EnvsafeConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.project.is_none());
        assert!(config.sync.is_none());
        assert!(config.team.is_empty());
        assert!(config.webhooks.is_none());
        assert_eq!(config.required.len(), 1);
    }
}
