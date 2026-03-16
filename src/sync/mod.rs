pub mod aws_ssm;
pub mod gcp;
pub mod onepassword;
pub mod vault_provider;

use anyhow::{bail, Result};

/// Configuration for a sync operation
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// The environment name (e.g., "dev", "staging", "prod")
    pub env: String,
    /// Prefix for parameter paths (e.g., "/myapp/dev/" for AWS SSM)
    pub prefix: Option<String>,
    /// Path (e.g., "secret/data/myapp" for Vault, or GCP project/secret)
    pub path: Option<String>,
    /// Vault name for 1Password
    pub vault_name: Option<String>,
}

/// Trait that all sync providers must implement
pub trait SyncProvider {
    /// Pull secrets from the remote provider and return them as key-value pairs
    fn pull(&self, config: &SyncConfig) -> Result<Vec<(String, String)>>;

    /// Push key-value pairs to the remote provider
    fn push(&self, config: &SyncConfig, vars: &[(String, String)]) -> Result<()>;

    /// Return the human-readable name of this provider
    fn name(&self) -> &str;
}

/// Get a sync provider by name
pub fn get_provider(name: &str) -> Result<Box<dyn SyncProvider>> {
    match name {
        "aws-ssm" => Ok(Box::new(aws_ssm::AwsSsmProvider)),
        "vault" => Ok(Box::new(vault_provider::VaultProvider)),
        "1password" => Ok(Box::new(onepassword::OnePasswordProvider)),
        "gcp" => Ok(Box::new(gcp::GcpProvider)),
        _ => bail!(
            "Unknown sync provider: '{}'. Supported providers: aws-ssm, vault, 1password, gcp",
            name
        ),
    }
}
