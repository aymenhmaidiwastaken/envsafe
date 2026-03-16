use anyhow::{bail, Context, Result};
use std::process::Command;

use super::{SyncConfig, SyncProvider};

/// 1Password sync provider.
/// Shells out to the `op` CLI tool.
pub struct OnePasswordProvider;

impl OnePasswordProvider {
    fn check_cli() -> Result<()> {
        let status = Command::new("op").arg("--version").output();
        match status {
            Ok(output) if output.status.success() => Ok(()),
            _ => bail!(
                "1Password CLI (op) is not installed or not in PATH.\n\
                 Install it from: https://developer.1password.com/docs/cli/get-started/\n\
                 Then sign in with: op signin"
            ),
        }
    }

    fn resolve_vault_name(config: &SyncConfig) -> String {
        if let Some(ref vault_name) = config.vault_name {
            vault_name.clone()
        } else {
            "envsafe".to_string()
        }
    }

    fn item_title(config: &SyncConfig) -> String {
        format!("envsafe-{}", config.env)
    }
}

impl SyncProvider for OnePasswordProvider {
    fn pull(&self, config: &SyncConfig) -> Result<Vec<(String, String)>> {
        Self::check_cli()?;
        let vault_name = Self::resolve_vault_name(config);
        let item_title = Self::item_title(config);

        // Get the item as JSON
        let output = Command::new("op")
            .args([
                "item",
                "get",
                &item_title,
                "--vault",
                &vault_name,
                "--format",
                "json",
            ])
            .output()
            .context("Failed to execute op CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If item doesn't exist, return empty
            if stderr.contains("not found") || stderr.contains("isn't an item") {
                return Ok(Vec::new());
            }
            bail!("1Password pull failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).context("Failed to parse op JSON output")?;

        let mut vars = Vec::new();

        // 1Password items have "fields" array with "label" and "value"
        if let Some(fields) = parsed.get("fields").and_then(|f| f.as_array()) {
            for field in fields {
                let label = field
                    .get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or_default();
                let value = field
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                // Skip internal 1Password fields (like "notesPlain", empty labels)
                if label.is_empty() || label == "notesPlain" {
                    continue;
                }

                // Skip fields in the default section that are 1Password metadata
                let section = field
                    .get("section")
                    .and_then(|s| s.get("label"))
                    .and_then(|l| l.as_str());

                // Only include fields from our "env" section or with no section
                if (section.is_none() || section == Some("env") || section == Some("Environment"))
                    && !value.is_empty()
                {
                    vars.push((label.to_string(), value.to_string()));
                }
            }
        }

        Ok(vars)
    }

    fn push(&self, config: &SyncConfig, vars: &[(String, String)]) -> Result<()> {
        Self::check_cli()?;
        let vault_name = Self::resolve_vault_name(config);
        let item_title = Self::item_title(config);

        // Check if item already exists
        let check = Command::new("op")
            .args([
                "item",
                "get",
                &item_title,
                "--vault",
                &vault_name,
                "--format",
                "json",
            ])
            .output();

        let item_exists = check.map(|o| o.status.success()).unwrap_or(false);

        if item_exists {
            // Delete existing item so we can recreate with fresh fields
            let _ = Command::new("op")
                .args(["item", "delete", &item_title, "--vault", &vault_name])
                .output();
        }

        // Build the item creation command with all variables as fields
        // op item create --category=SecureNote --title=TITLE --vault=VAULT key[text]=value ...
        let mut args = vec![
            "item".to_string(),
            "create".to_string(),
            "--category=SecureNote".to_string(),
            format!("--title={}", item_title),
            format!("--vault={}", vault_name),
        ];

        for (key, value) in vars {
            // Use the field assignment syntax: 'key[text]=value'
            args.push(format!("{}[text]={}", key, value));
        }

        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = Command::new("op")
            .args(&str_args)
            .output()
            .context("Failed to execute op CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("1Password push failed: {}", stderr.trim());
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "1password"
    }
}
