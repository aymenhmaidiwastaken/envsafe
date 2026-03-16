use anyhow::Result;

use crate::config;

pub fn execute(key: &str, env: &str) -> Result<()> {
    let config = config::find_project_root()?;
    let vault = crate::vault::Vault::load(&config)?;

    match vault.get(env, key)? {
        Some(entry) => {
            println!("{}", entry.value);
        }
        None => {
            anyhow::bail!("{} not found in [{}]", key, env);
        }
    }

    Ok(())
}
