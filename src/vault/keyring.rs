use anyhow::{Context, Result};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;

use crate::config::{self, ProjectConfig};

/// Magic header to identify passphrase-protected key files
const PASSPHRASE_MAGIC: &[u8] = b"ENVSAFE_PP_V1\n";
const ARGON2_SALT_LEN: usize = 16;

/// Generate and store a new master key for a project.
/// Stores in both the key file and OS keychain.
pub fn create_key(config: &ProjectConfig) -> Result<Vec<u8>> {
    let key = super::crypto::generate_key();
    let key_path = key_file_path(&config.project_id)?;
    let encoded = STANDARD.encode(&key);
    std::fs::write(&key_path, &encoded)?;

    // Also store in OS keychain (best-effort)
    let _ = store_in_keychain(&config.project_id, &key);

    Ok(key)
}

/// Generate and store a new master key protected by a passphrase.
pub fn create_key_with_passphrase(config: &ProjectConfig) -> Result<Vec<u8>> {
    let master_key = super::crypto::generate_key();

    let passphrase =
        rpassword::prompt_password("Enter passphrase: ").context("Failed to read passphrase")?;
    let passphrase_confirm = rpassword::prompt_password("Confirm passphrase: ")
        .context("Failed to read passphrase confirmation")?;

    if passphrase != passphrase_confirm {
        anyhow::bail!("Passphrases do not match");
    }

    // Generate a random salt for Argon2
    let mut salt_bytes = [0u8; ARGON2_SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt_bytes);

    // Derive an encryption key from the passphrase
    let derived_key = derive_key_from_passphrase(&passphrase, &salt_bytes)?;

    // Encrypt the master key with the derived key
    let encrypted_master = super::crypto::encrypt(&derived_key, &master_key)?;

    // Build the file: MAGIC + salt + encrypted_master_key
    let mut file_data = Vec::new();
    file_data.extend_from_slice(PASSPHRASE_MAGIC);
    file_data.extend_from_slice(&salt_bytes);
    file_data.extend_from_slice(&encrypted_master);

    let key_path = key_file_path(&config.project_id)?;
    let encoded = STANDARD.encode(&file_data);
    std::fs::write(&key_path, &encoded)?;

    // Also store in OS keychain (best-effort)
    let _ = store_in_keychain(&config.project_id, &master_key);

    Ok(master_key)
}

/// Load the master key for a project.
/// Tries OS keychain first, then falls back to file-based keys.
/// Automatically detects passphrase-protected key files.
pub fn load_key(config: &ProjectConfig) -> Result<Vec<u8>> {
    // Try OS keychain first
    if let Ok(key) = load_from_keychain(&config.project_id) {
        return Ok(key);
    }

    // Fall back to file-based key
    let key_path = key_file_path(&config.project_id)?;
    let encoded = std::fs::read_to_string(&key_path).with_context(|| {
        format!(
            "No key found for project {}. Run `envsafe init` or `envsafe key import`.",
            config.project_id
        )
    })?;

    let raw = STANDARD
        .decode(encoded.trim())
        .context("Invalid key encoding")?;

    // Check if this is a passphrase-protected key
    if raw.starts_with(PASSPHRASE_MAGIC) {
        load_key_with_passphrase_from_data(&raw)
    } else {
        // Plain key (backward compatible)
        Ok(raw)
    }
}

/// Load a passphrase-protected key by prompting for the passphrase.
pub fn load_key_with_passphrase(config: &ProjectConfig) -> Result<Vec<u8>> {
    let key_path = key_file_path(&config.project_id)?;
    let encoded = std::fs::read_to_string(&key_path).with_context(|| {
        format!(
            "No key found for project {}. Run `envsafe init` or `envsafe key import`.",
            config.project_id
        )
    })?;

    let raw = STANDARD
        .decode(encoded.trim())
        .context("Invalid key encoding")?;

    if !raw.starts_with(PASSPHRASE_MAGIC) {
        anyhow::bail!("Key file is not passphrase-protected");
    }

    load_key_with_passphrase_from_data(&raw)
}

/// Internal: decrypt a passphrase-protected key from its raw bytes.
fn load_key_with_passphrase_from_data(raw: &[u8]) -> Result<Vec<u8>> {
    let data = &raw[PASSPHRASE_MAGIC.len()..];

    if data.len() < ARGON2_SALT_LEN {
        anyhow::bail!("Corrupted passphrase-protected key file");
    }

    let (salt_bytes, encrypted_master) = data.split_at(ARGON2_SALT_LEN);

    let passphrase =
        rpassword::prompt_password("Enter passphrase: ").context("Failed to read passphrase")?;

    let derived_key = derive_key_from_passphrase(&passphrase, salt_bytes)?;
    let master_key = super::crypto::decrypt(&derived_key, encrypted_master)
        .context("Wrong passphrase or corrupted key file")?;

    Ok(master_key)
}

/// Derive a 32-byte encryption key from a passphrase and salt using Argon2id.
fn derive_key_from_passphrase(passphrase: &str, salt_bytes: &[u8]) -> Result<Vec<u8>> {
    let argon2 = Argon2::default(); // Argon2id v19 with default params
    let mut output_key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt_bytes, &mut output_key)
        .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;
    Ok(output_key.to_vec())
}

/// Store the master key in the OS keychain.
pub fn store_in_keychain(project_id: &str, key: &[u8]) -> Result<()> {
    let entry =
        ::keyring::Entry::new("envsafe", project_id).context("Failed to create keychain entry")?;
    let encoded = STANDARD.encode(key);
    entry
        .set_password(&encoded)
        .context("Failed to store key in OS keychain")?;
    Ok(())
}

/// Load the master key from the OS keychain.
pub fn load_from_keychain(project_id: &str) -> Result<Vec<u8>> {
    let entry =
        ::keyring::Entry::new("envsafe", project_id).context("Failed to access keychain entry")?;
    let encoded = entry
        .get_password()
        .context("Key not found in OS keychain")?;
    let key = STANDARD
        .decode(encoded.trim())
        .context("Invalid key encoding in keychain")?;
    Ok(key)
}

/// Export the project key as a base64 string
pub fn export_key(config: &ProjectConfig) -> Result<String> {
    let key_path = key_file_path(&config.project_id)?;
    let encoded = std::fs::read_to_string(&key_path).context("No key found for this project")?;
    // Format: project_id:base64_key
    Ok(format!("{}:{}", config.project_id, encoded.trim()))
}

/// Import a project key from a team member
pub fn import_key(config: &ProjectConfig, key_string: &str) -> Result<()> {
    let parts: Vec<&str> = key_string.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid key format. Expected format: project_id:base64_key");
    }

    let project_id = parts[0];
    let key_data = parts[1];

    // Validate the key data is valid base64
    let key_bytes = STANDARD
        .decode(key_data.trim())
        .context("Invalid key data")?;

    // Verify project ID matches if we have one
    if project_id != config.project_id {
        anyhow::bail!(
            "Key is for project {} but current project is {}",
            project_id,
            config.project_id
        );
    }

    let key_path = key_file_path(project_id)?;
    std::fs::write(&key_path, key_data.trim())?;

    // Also store in OS keychain (best-effort)
    let _ = store_in_keychain(project_id, &key_bytes);

    Ok(())
}

fn key_file_path(project_id: &str) -> Result<std::path::PathBuf> {
    let keys_dir = config::keys_dir()?;
    Ok(keys_dir.join(format!("{}.key", project_id)))
}
