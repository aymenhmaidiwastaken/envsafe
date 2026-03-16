use anyhow::Result;

use crate::config;

pub fn execute(key: &str, value: &str, env: &str, secret: bool) -> Result<()> {
    let config = config::find_project_root()?;
    let mut vault = crate::vault::Vault::load(&config)?;
    vault.set(env, key, value, secret)?;

    let label = if secret { " (secret)" } else { "" };
    println!("Set {}{} in [{}]", key, label, env);
    Ok(())
}
