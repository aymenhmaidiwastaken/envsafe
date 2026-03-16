use anyhow::{bail, Context, Result};
use std::process::Command;

use super::{SyncConfig, SyncProvider};

/// AWS SSM Parameter Store sync provider.
/// Shells out to the `aws` CLI tool.
pub struct AwsSsmProvider;

impl AwsSsmProvider {
    fn check_cli() -> Result<()> {
        let status = Command::new("aws").arg("--version").output();
        match status {
            Ok(output) if output.status.success() => Ok(()),
            _ => bail!(
                "AWS CLI is not installed or not in PATH.\n\
                 Install it from: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html\n\
                 Then configure with: aws configure"
            ),
        }
    }

    fn resolve_prefix(config: &SyncConfig) -> Result<String> {
        if let Some(ref prefix) = config.prefix {
            let p = if prefix.ends_with('/') {
                prefix.clone()
            } else {
                format!("{}/", prefix)
            };
            Ok(p)
        } else {
            Ok(format!("/envsafe/{}/", config.env))
        }
    }
}

impl SyncProvider for AwsSsmProvider {
    fn pull(&self, config: &SyncConfig) -> Result<Vec<(String, String)>> {
        Self::check_cli()?;
        let prefix = Self::resolve_prefix(config)?;

        let output = Command::new("aws")
            .args([
                "ssm",
                "get-parameters-by-path",
                "--path",
                &prefix,
                "--with-decryption",
                "--query",
                "Parameters[*].[Name,Value]",
                "--output",
                "text",
            ])
            .output()
            .context("Failed to execute aws CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("AWS SSM pull failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut vars = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Output format is tab-separated: Name\tValue
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() == 2 {
                // Strip the prefix from the parameter name to get the key
                let full_name = parts[0].trim();
                let value = parts[1].trim();
                let key = full_name
                    .strip_prefix(&prefix)
                    .unwrap_or(full_name)
                    .to_string();
                if !key.is_empty() {
                    vars.push((key, value.to_string()));
                }
            }
        }

        Ok(vars)
    }

    fn push(&self, config: &SyncConfig, vars: &[(String, String)]) -> Result<()> {
        Self::check_cli()?;
        let prefix = Self::resolve_prefix(config)?;

        for (key, value) in vars {
            let param_name = format!("{}{}", prefix, key);
            let output = Command::new("aws")
                .args([
                    "ssm",
                    "put-parameter",
                    "--name",
                    &param_name,
                    "--value",
                    value,
                    "--type",
                    "SecureString",
                    "--overwrite",
                ])
                .output()
                .context("Failed to execute aws CLI")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "AWS SSM push failed for parameter '{}': {}",
                    key,
                    stderr.trim()
                );
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "aws-ssm"
    }
}
