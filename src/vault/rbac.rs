use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use super::crypto;
use crate::config::{self, ProjectConfig};

/// Associates an environment with a specific encryption key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentKey {
    pub env: String,
    pub key_id: String,
}

/// Create a separate encryption key for a specific environment.
/// The key is stored in the keys directory with a name based on project_id and env.
pub fn create_env_key(config: &ProjectConfig, env: &str) -> Result<Vec<u8>> {
    let key = crypto::generate_key();
    let key_id = format!("{}_{}", config.project_id, env);
    let key_path = env_key_file_path(&config.project_id, env)?;
    let encoded = STANDARD.encode(&key);
    std::fs::write(&key_path, &encoded)?;

    // Store metadata about the environment key
    let env_key = EnvironmentKey {
        env: env.to_string(),
        key_id,
    };
    append_env_key_metadata(config, &env_key)?;

    // Also store in OS keychain (best-effort)
    let keychain_id = format!("{}_{}", config.project_id, env);
    let _ = super::keyring::store_in_keychain(&keychain_id, &key);

    Ok(key)
}

/// Load the encryption key for a specific environment.
/// Falls back to the master key if no environment-specific key exists.
pub fn load_env_key(config: &ProjectConfig, env: &str) -> Result<Vec<u8>> {
    // Try environment-specific key first
    let key_path = env_key_file_path(&config.project_id, env)?;
    if key_path.exists() {
        let encoded =
            std::fs::read_to_string(&key_path).context("Failed to read environment key file")?;
        let key = STANDARD
            .decode(encoded.trim())
            .context("Invalid environment key encoding")?;
        return Ok(key);
    }

    // Try OS keychain for env-specific key
    let keychain_id = format!("{}_{}", config.project_id, env);
    if let Ok(key) = super::keyring::load_from_keychain(&keychain_id) {
        return Ok(key);
    }

    // Fall back to master key
    super::keyring::load_key(config)
}

/// Get the file path for an environment-specific key.
fn env_key_file_path(project_id: &str, env: &str) -> Result<std::path::PathBuf> {
    let keys_dir = config::keys_dir()?;
    Ok(keys_dir.join(format!("{}_{}.key", project_id, env)))
}

/// Append environment key metadata to the project's env keys manifest.
fn append_env_key_metadata(config: &ProjectConfig, env_key: &EnvironmentKey) -> Result<()> {
    let manifest_path = config.envsafe_dir().join("env_keys.json");

    let mut keys: Vec<EnvironmentKey> = if manifest_path.exists() {
        let data =
            std::fs::read_to_string(&manifest_path).context("Failed to read env keys manifest")?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Replace existing entry for this env, or add new
    keys.retain(|k| k.env != env_key.env);
    keys.push(env_key.clone());

    let json = serde_json::to_string_pretty(&keys)?;
    std::fs::write(&manifest_path, json)?;

    Ok(())
}
