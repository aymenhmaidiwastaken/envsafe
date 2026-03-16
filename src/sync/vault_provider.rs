use anyhow::{bail, Context, Result};
use std::process::Command;

use super::{SyncConfig, SyncProvider};

/// HashiCorp Vault sync provider.
/// Shells out to the `vault` CLI tool.
pub struct VaultProvider;

impl VaultProvider {
    fn check_cli() -> Result<()> {
        let status = Command::new("vault").arg("version").output();
        match status {
            Ok(output) if output.status.success() => Ok(()),
            _ => bail!(
                "HashiCorp Vault CLI is not installed or not in PATH.\n\
                 Install it from: https://developer.hashicorp.com/vault/install\n\
                 Then set VAULT_ADDR and authenticate with: vault login"
            ),
        }
    }

    fn resolve_path(config: &SyncConfig) -> String {
        if let Some(ref path) = config.path {
            path.clone()
        } else {
            format!("secret/data/envsafe/{}", config.env)
        }
    }
}

impl SyncProvider for VaultProvider {
    fn pull(&self, config: &SyncConfig) -> Result<Vec<(String, String)>> {
        Self::check_cli()?;
        let path = Self::resolve_path(config);

        // Use vault kv get with -format=json to get structured output
        let output = Command::new("vault")
            .args(["kv", "get", "-format=json", &path])
            .output()
            .context("Failed to execute vault CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Vault pull failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).context("Failed to parse vault JSON output")?;

        let mut vars = Vec::new();

        // vault kv get -format=json returns { "data": { "data": { ... } } } for KV v2
        // or { "data": { ... } } for KV v1
        let data = parsed.get("data").and_then(|d| d.get("data").or(Some(d)));

        if let Some(data) = data {
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    let val_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    vars.push((key.clone(), val_str));
                }
            }
        }

        Ok(vars)
    }

    fn push(&self, config: &SyncConfig, vars: &[(String, String)]) -> Result<()> {
        Self::check_cli()?;
        let path = Self::resolve_path(config);

        // Build key=value arguments for vault kv put
        let mut args = vec!["kv".to_string(), "put".to_string(), path];
        for (key, value) in vars {
            args.push(format!("{}={}", key, value));
        }

        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = Command::new("vault")
            .args(&str_args)
            .output()
            .context("Failed to execute vault CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Vault push failed: {}", stderr.trim());
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "vault"
    }
}
