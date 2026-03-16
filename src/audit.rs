use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::ProjectConfig;
use crate::vault::{crypto, keyring};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub user: String,
}

/// Log an action to the encrypted audit log.
///
/// The audit log is stored as encrypted JSON lines in `.envsafe/audit.log`.
/// Each call appends a new encrypted entry.
pub fn log_action(
    config: &ProjectConfig,
    action: &str,
    env: Option<&str>,
    key: Option<&str>,
) -> Result<()> {
    let master_key = keyring::load_key(config)?;
    let audit_path = config.envsafe_dir().join("audit.log");

    let entry = AuditEntry {
        timestamp: Utc::now().to_rfc3339(),
        action: action.to_string(),
        env: env.map(|s| s.to_string()),
        key: key.map(|s| s.to_string()),
        user: get_current_user(),
    };

    let entry_json = serde_json::to_string(&entry)?;
    let encrypted = crypto::encrypt(&master_key, entry_json.as_bytes())?;
    let encoded = STANDARD.encode(&encrypted);

    // Append the encrypted line to the audit log
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&audit_path)
        .context("Failed to open audit log")?;
    writeln!(file, "{}", encoded)?;

    Ok(())
}

/// Read and decrypt the entire audit log.
pub fn read_audit_log(config: &ProjectConfig) -> Result<Vec<AuditEntry>> {
    let master_key = keyring::load_key(config)?;
    let audit_path = config.envsafe_dir().join("audit.log");

    if !audit_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&audit_path).context("Failed to read audit log")?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let encrypted = STANDARD
            .decode(line)
            .context("Invalid audit log entry encoding")?;
        let decrypted = crypto::decrypt(&master_key, &encrypted)
            .context("Failed to decrypt audit log entry")?;
        let entry: AuditEntry =
            serde_json::from_slice(&decrypted).context("Failed to parse audit log entry")?;
        entries.push(entry);
    }

    Ok(entries)
}

/// Get the current system username.
fn get_current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
