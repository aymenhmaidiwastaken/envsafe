use anyhow::{bail, Context, Result};
use std::process::Command;

use super::{SyncConfig, SyncProvider};

/// Google Cloud Secret Manager sync provider.
/// Shells out to the `gcloud` CLI tool.
pub struct GcpProvider;

impl GcpProvider {
    fn check_cli() -> Result<()> {
        let status = Command::new("gcloud").arg("version").output();
        match status {
            Ok(output) if output.status.success() => Ok(()),
            _ => bail!(
                "Google Cloud CLI (gcloud) is not installed or not in PATH.\n\
                 Install it from: https://cloud.google.com/sdk/docs/install\n\
                 Then authenticate with: gcloud auth login"
            ),
        }
    }

    /// Resolve the GCP project. Uses --path if provided, otherwise tries
    /// to get the current project from gcloud config.
    fn resolve_project(config: &SyncConfig) -> Result<String> {
        if let Some(ref path) = config.path {
            // If path contains a '/', treat the first segment as the project
            if let Some((project, _)) = path.split_once('/') {
                return Ok(project.to_string());
            }
            return Ok(path.clone());
        }

        // Fall back to current gcloud project
        let output = Command::new("gcloud")
            .args(["config", "get-value", "project"])
            .output()
            .context("Failed to get gcloud project")?;

        if !output.status.success() {
            bail!(
                "No GCP project specified. Use --path PROJECT_ID or set via:\n\
                 gcloud config set project PROJECT_ID"
            );
        }

        let project = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if project.is_empty() || project == "(unset)" {
            bail!(
                "No GCP project configured. Use --path PROJECT_ID or set via:\n\
                 gcloud config set project PROJECT_ID"
            );
        }

        Ok(project)
    }

    /// Build the secret name prefix for this environment
    fn secret_prefix(config: &SyncConfig) -> String {
        format!("envsafe-{}-", config.env)
    }
}

impl SyncProvider for GcpProvider {
    fn pull(&self, config: &SyncConfig) -> Result<Vec<(String, String)>> {
        Self::check_cli()?;
        let project = Self::resolve_project(config)?;
        let prefix = Self::secret_prefix(config);

        // List secrets with the prefix filter
        let output = Command::new("gcloud")
            .args([
                "secrets",
                "list",
                "--project",
                &project,
                "--filter",
                &format!("name~^projects/.*/secrets/{}.*", prefix),
                "--format",
                "value(name)",
            ])
            .output()
            .context("Failed to execute gcloud CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("GCP Secret Manager list failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut vars = Vec::new();

        for line in stdout.lines() {
            let secret_name = line.trim();
            if secret_name.is_empty() {
                continue;
            }

            // Access the latest version of the secret
            let value_output = Command::new("gcloud")
                .args([
                    "secrets",
                    "versions",
                    "access",
                    "latest",
                    "--secret",
                    secret_name,
                    "--project",
                    &project,
                ])
                .output()
                .context(format!("Failed to access secret '{}'", secret_name))?;

            if !value_output.status.success() {
                let stderr = String::from_utf8_lossy(&value_output.stderr);
                eprintln!(
                    "Warning: Could not access secret '{}': {}",
                    secret_name,
                    stderr.trim()
                );
                continue;
            }

            let value = String::from_utf8_lossy(&value_output.stdout).to_string();

            // Strip the prefix to get the original variable name
            let key = secret_name
                .strip_prefix(&prefix)
                .unwrap_or(secret_name)
                .to_string();

            if !key.is_empty() {
                vars.push((key, value));
            }
        }

        Ok(vars)
    }

    fn push(&self, config: &SyncConfig, vars: &[(String, String)]) -> Result<()> {
        Self::check_cli()?;
        let project = Self::resolve_project(config)?;
        let prefix = Self::secret_prefix(config);

        for (key, value) in vars {
            let secret_name = format!("{}{}", prefix, key);

            // Try to create the secret first (idempotent - may already exist)
            let create_output = Command::new("gcloud")
                .args([
                    "secrets",
                    "create",
                    &secret_name,
                    "--project",
                    &project,
                    "--replication-policy",
                    "automatic",
                ])
                .output()
                .context("Failed to execute gcloud CLI")?;

            // Ignore "already exists" errors
            if !create_output.status.success() {
                let stderr = String::from_utf8_lossy(&create_output.stderr);
                if !stderr.contains("ALREADY_EXISTS") {
                    bail!(
                        "GCP Secret Manager create failed for '{}': {}",
                        key,
                        stderr.trim()
                    );
                }
            }

            // Add a new version with the value, piped via stdin
            let mut child = Command::new("gcloud")
                .args([
                    "secrets",
                    "versions",
                    "add",
                    &secret_name,
                    "--project",
                    &project,
                    "--data-file",
                    "-",
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn gcloud CLI")?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(value.as_bytes())?;
            }

            let output = child.wait_with_output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "GCP Secret Manager push failed for '{}': {}",
                    key,
                    stderr.trim()
                );
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "gcp"
    }
}
