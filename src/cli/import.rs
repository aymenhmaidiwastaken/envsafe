use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::config;
use crate::env::parser;

/// Import variables from an existing .env file into the vault
pub fn execute(file: &str, env: &str, secret: bool) -> Result<()> {
    let config = config::find_project_root()?;
    let mut vault = crate::vault::Vault::load(&config)?;

    let path = PathBuf::from(file);
    let vars = parser::parse_dotenv(&path).with_context(|| format!("Failed to read {}", file))?;

    if vars.is_empty() {
        println!("No variables found in {}", file);
        return Ok(());
    }

    let mut count = 0;
    for (key, value) in &vars {
        vault.set(env, key, value, secret)?;
        count += 1;
    }

    println!(
        "{} Imported {} variable(s) from {} into [{}]",
        "done!".green().bold(),
        count,
        file,
        env
    );

    Ok(())
}
