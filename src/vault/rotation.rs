use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;

use super::{crypto, keyring, Vault};
use crate::config::{self, ProjectConfig};

/// Rotate the master key for a project.
///
/// 1. Loads the vault with the old key
/// 2. Generates a new master key
/// 3. Re-encrypts the vault data with the new key
/// 4. Saves the new key (and updates keychain)
/// 5. Backs up the old key file with a timestamp suffix
pub fn rotate_key(config: &ProjectConfig) -> Result<()> {
    // Load the old key
    let old_key = keyring::load_key(config)?;

    // Load the vault with the old key
    let vault = Vault::load_with_key(config, &old_key)?;

    // Generate a new master key
    let new_key = crypto::generate_key();

    // Backup the old key file with a timestamp suffix
    let keys_dir = config::keys_dir()?;
    let old_key_path = keys_dir.join(format!("{}.key", config.project_id));
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let backup_path = keys_dir.join(format!("{}.key.{}", config.project_id, timestamp));

    if old_key_path.exists() {
        std::fs::copy(&old_key_path, &backup_path)
            .with_context(|| format!("Failed to backup old key to {:?}", backup_path))?;
    }

    // Save the new key to file
    let encoded = STANDARD.encode(&new_key);
    std::fs::write(&old_key_path, &encoded).context("Failed to write new key file")?;

    // Update OS keychain (best-effort)
    let _ = keyring::store_in_keychain(&config.project_id, &new_key);

    // Re-encrypt and save the vault with the new key
    vault.save_with_key(&new_key)?;

    eprintln!(
        "Key rotated successfully. Old key backed up to {:?}",
        backup_path
    );

    Ok(())
}
