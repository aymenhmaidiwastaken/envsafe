use anyhow::Result;

use crate::config;
use crate::env::injector;

pub fn execute(env: &str, cmd: &[String]) -> Result<()> {
    let config = config::find_project_root()?;
    let vault = crate::vault::Vault::load(&config)?;
    let vars = vault.get_env_vars(env)?;

    let exit_code = injector::run_with_env(&vars, cmd)?;
    std::process::exit(exit_code);
}
