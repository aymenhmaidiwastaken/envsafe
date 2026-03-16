pub mod crypto;
pub mod keyring;
pub mod rbac;
pub mod rotation;
pub mod store;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::config::ProjectConfig;

/// A single environment variable entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarEntry {
    pub value: String,
    #[serde(default)]
    pub secret: bool,
    /// Optional expiry timestamp in ISO 8601 format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// All variables for one environment
pub type EnvStore = BTreeMap<String, VarEntry>;

/// The full vault: multiple environments, each with key-value pairs
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct VaultData {
    pub environments: BTreeMap<String, EnvStore>,
}

impl VaultData {
    pub fn new() -> Self {
        Self::default()
    }
}

/// High-level vault handle
pub struct Vault {
    pub data: VaultData,
    config: ProjectConfig,
    master_key: Vec<u8>,
}

impl Vault {
    /// Load (or create) the vault for a project
    pub fn load(config: &ProjectConfig) -> Result<Self> {
        let master_key = keyring::load_key(config)?;
        let vault_path = config.vault_path();

        let data = if vault_path.exists() {
            let ciphertext = std::fs::read(&vault_path)
                .with_context(|| format!("Failed to read vault at {:?}", vault_path))?;
            let plaintext = crypto::decrypt(&master_key, &ciphertext)?;
            serde_json::from_slice(&plaintext).context("Failed to parse vault data")?
        } else {
            VaultData::new()
        };

        Ok(Self {
            data,
            config: ProjectConfig {
                project_id: config.project_id.clone(),
                project_root: config.project_root.clone(),
                created_at: config.created_at.clone(),
            },
            master_key,
        })
    }

    /// Load the vault with a specific key (used by key rotation)
    pub fn load_with_key(config: &ProjectConfig, key: &[u8]) -> Result<Self> {
        let vault_path = config.vault_path();

        let data = if vault_path.exists() {
            let ciphertext = std::fs::read(&vault_path)
                .with_context(|| format!("Failed to read vault at {:?}", vault_path))?;
            let plaintext = crypto::decrypt(key, &ciphertext)?;
            serde_json::from_slice(&plaintext).context("Failed to parse vault data")?
        } else {
            VaultData::new()
        };

        Ok(Self {
            data,
            config: ProjectConfig {
                project_id: config.project_id.clone(),
                project_root: config.project_root.clone(),
                created_at: config.created_at.clone(),
            },
            master_key: key.to_vec(),
        })
    }

    /// Save the vault to disk (encrypted)
    pub fn save(&self) -> Result<()> {
        let plaintext = serde_json::to_vec_pretty(&self.data)?;
        let ciphertext = crypto::encrypt(&self.master_key, &plaintext)?;
        std::fs::write(self.config.vault_path(), ciphertext)?;
        Ok(())
    }

    /// Save the vault with a specific key (used by key rotation)
    pub fn save_with_key(&self, key: &[u8]) -> Result<()> {
        let plaintext = serde_json::to_vec_pretty(&self.data)?;
        let ciphertext = crypto::encrypt(key, &plaintext)?;
        std::fs::write(self.config.vault_path(), ciphertext)?;
        Ok(())
    }

    /// Set a variable in an environment
    pub fn set(&mut self, env: &str, key: &str, value: &str, secret: bool) -> Result<()> {
        let env_store = self.data.environments.entry(env.to_string()).or_default();
        env_store.insert(
            key.to_string(),
            VarEntry {
                value: value.to_string(),
                secret,
                expires_at: None,
            },
        );
        self.save()
    }

    /// Set a variable with an expiry time
    pub fn set_with_expiry(
        &mut self,
        env: &str,
        key: &str,
        value: &str,
        secret: bool,
        expires_at: Option<String>,
    ) -> Result<()> {
        let env_store = self.data.environments.entry(env.to_string()).or_default();
        env_store.insert(
            key.to_string(),
            VarEntry {
                value: value.to_string(),
                secret,
                expires_at,
            },
        );
        self.save()
    }

    /// Get a variable from an environment.
    /// Prints a warning if the variable has expired, but still returns the value.
    pub fn get(&self, env: &str, key: &str) -> Result<Option<&VarEntry>> {
        let entry = self
            .data
            .environments
            .get(env)
            .and_then(|store| store.get(key));

        if let Some(var) = &entry {
            if let Some(ref expires_at) = var.expires_at {
                if is_expired(expires_at) {
                    eprintln!(
                        "WARNING: Variable '{}' in environment '{}' expired at {}",
                        key, env, expires_at
                    );
                }
            }
        }

        Ok(entry)
    }

    /// Check all variables in an environment for expiry.
    /// Returns a list of (key, expires_at) for expired variables.
    pub fn check_expired(&self, env: &str) -> Vec<(String, String)> {
        let mut expired = Vec::new();
        if let Some(store) = self.data.environments.get(env) {
            for (key, entry) in store {
                if let Some(ref expires_at) = entry.expires_at {
                    if is_expired(expires_at) {
                        expired.push((key.clone(), expires_at.clone()));
                    }
                }
            }
        }
        expired
    }

    /// Remove a variable from an environment
    pub fn remove(&mut self, env: &str, key: &str) -> Result<()> {
        if let Some(store) = self.data.environments.get_mut(env) {
            store.remove(key);
        }
        self.save()
    }

    /// List all variables in an environment: (key, value, is_secret)
    pub fn list(&self, env: &str) -> Result<Vec<(String, String, bool)>> {
        let entries = self
            .data
            .environments
            .get(env)
            .map(|store| {
                store
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value.clone(), v.secret))
                    .collect()
            })
            .unwrap_or_default();
        Ok(entries)
    }

    /// List all environment names
    pub fn environments(&self) -> Vec<String> {
        self.data.environments.keys().cloned().collect()
    }

    /// Get all variables for an environment as key-value pairs
    pub fn get_env_vars(&self, env: &str) -> Result<Vec<(String, String)>> {
        let entries = self
            .data
            .environments
            .get(env)
            .map(|store| {
                store
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value.clone()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(entries)
    }

    /// Get a reference to the config
    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    /// Get a reference to the master key
    pub fn master_key(&self) -> &[u8] {
        &self.master_key
    }

    /// Lock: export encrypted vault file (.env.vault) for git sharing
    pub fn lock(&self) -> Result<()> {
        let plaintext = serde_json::to_vec_pretty(&self.data)?;
        let ciphertext = crypto::encrypt(&self.master_key, &plaintext)?;
        let vault_file = self.config.project_root.join(".env.vault");
        let encoded = STANDARD.encode(&ciphertext);
        std::fs::write(&vault_file, encoded)?;
        Ok(())
    }

    /// Unlock: import from .env.vault file
    pub fn unlock(&mut self) -> Result<()> {
        let vault_file = self.config.project_root.join(".env.vault");
        let encoded = std::fs::read_to_string(&vault_file)
            .context("No .env.vault file found. Run `envsafe lock` first.")?;
        let ciphertext = STANDARD.decode(encoded.trim())?;
        let plaintext = crypto::decrypt(&self.master_key, &ciphertext)?;
        self.data = serde_json::from_slice(&plaintext)?;
        self.save()?;
        Ok(())
    }
}

/// Check if a timestamp string (ISO 8601) is in the past.
fn is_expired(expires_at: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(expires_at) {
        Ok(dt) => Utc::now() > dt,
        Err(_) => false, // If we can't parse, don't treat as expired
    }
}
