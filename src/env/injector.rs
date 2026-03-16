use anyhow::Result;
use std::process::Command;

/// Run a command with environment variables injected
pub fn run_with_env(vars: &[(String, String)], cmd: &[String]) -> Result<i32> {
    if cmd.is_empty() {
        anyhow::bail!("No command specified");
    }

    let program = &cmd[0];
    let args = &cmd[1..];

    let mut command = Command::new(program);
    command.args(args);

    // Inject environment variables
    for (key, value) in vars {
        command.env(key, value);
    }

    let status = command
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", program, e))?;

    Ok(status.code().unwrap_or(1))
}
