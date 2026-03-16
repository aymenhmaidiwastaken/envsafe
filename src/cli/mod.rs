pub mod diff;
pub mod export;
pub mod get;
pub mod hook_shell;
pub mod import;
pub mod init;
pub mod run;
pub mod scan;
pub mod template;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config;

#[derive(Parser)]
#[command(
    name = "envsafe",
    about = "Your secrets, encrypted, everywhere. One tool for all .env management.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Enable debug logging
    #[arg(long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize envsafe in the current project
    Init,

    /// Set an environment variable
    Set {
        /// Variable name
        key: String,
        /// Variable value
        value: String,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
        /// Mark as a sensitive secret
        #[arg(long)]
        secret: bool,
        /// Set expiry (e.g. "30d", "24h", "2024-12-31")
        #[arg(long)]
        expires: Option<String>,
    },

    /// Get an environment variable
    Get {
        /// Variable name
        key: String,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Remove an environment variable
    Rm {
        /// Variable name
        key: String,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// List all environment variables
    Ls {
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
        /// Show actual values (default: masked)
        #[arg(long)]
        show: bool,
    },

    /// Run a command with injected environment variables
    Run {
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
        /// Command and arguments to run
        #[arg(trailing_var_arg = true, required = true)]
        cmd: Vec<String>,
    },

    /// Export environment variables
    Export {
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
        /// Output format: shell, dotenv, json, docker, kubernetes
        #[arg(long, default_value = "shell")]
        format: String,
    },

    /// Import variables from an existing .env file
    Import {
        /// Path to the .env file to import
        file: String,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
        /// Mark all imported variables as secrets
        #[arg(long)]
        secret: bool,
    },

    /// List all environments
    Envs,

    /// Compare two environments (enhanced color diff)
    Diff {
        /// First environment
        env1: String,
        /// Second environment
        env2: String,
        /// Show actual values (default: masked)
        #[arg(long)]
        show: bool,
    },

    /// Lock secrets into encrypted vault file for git
    Lock,

    /// Unlock secrets from vault file
    Unlock,

    /// Manage project encryption keys
    #[command(subcommand)]
    Key(KeyCommands),

    /// Validate environment against schema
    Validate {
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Manage git pre-commit hook
    Hook {
        #[command(subcommand)]
        action: HookCommands,
    },

    /// Scan repository for leaked secrets
    Scan,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Print shell hook for auto-injection (eval "$(envsafe hook-shell bash)")
    HookShell {
        /// Shell type: bash, zsh, fish
        shell: String,
    },

    /// Generate man page
    ManPage,

    /// Pull secrets from a cloud provider into the local vault
    Pull {
        /// Provider name: aws-ssm, vault, 1password, gcp
        provider: String,
        /// Prefix for parameter paths (used by aws-ssm)
        #[arg(long)]
        prefix: Option<String>,
        /// Path for secrets (used by vault, gcp)
        #[arg(long)]
        path: Option<String>,
        /// Vault name (used by 1password)
        #[arg(long)]
        vault_name: Option<String>,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Push secrets from the local vault to a cloud provider
    Push {
        /// Provider name: aws-ssm, vault, 1password, gcp
        provider: String,
        /// Prefix for parameter paths (used by aws-ssm)
        #[arg(long)]
        prefix: Option<String>,
        /// Path for secrets (used by vault, gcp)
        #[arg(long)]
        path: Option<String>,
        /// Vault name (used by 1password)
        #[arg(long)]
        vault_name: Option<String>,
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Generate a .env.example template file
    Template {
        /// Target environment (default: dev)
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Open interactive TUI mode
    #[command(name = "ui")]
    Tui,

    /// Rotate the project encryption key
    RotateKey,

    /// View audit log
    Audit {
        /// Number of recent entries to show
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Manage telemetry settings
    Telemetry {
        #[command(subcommand)]
        action: TelemetryCommands,
    },

    /// Run a plugin
    Plugin {
        /// Plugin name (looks for envsafe-plugin-<name> in PATH)
        name: String,
        /// Arguments to pass to the plugin
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// List available plugins
    Plugins,
}

#[derive(Subcommand)]
pub enum KeyCommands {
    /// Export the project key for sharing
    Export,
    /// Import a project key from a team member
    Import {
        /// The key to import
        key: String,
    },
}

#[derive(Subcommand)]
pub enum HookCommands {
    /// Install pre-commit hook
    Install,
    /// Uninstall pre-commit hook
    Uninstall,
}

#[derive(Subcommand)]
pub enum TelemetryCommands {
    /// Enable telemetry
    Enable,
    /// Disable telemetry
    Disable,
    /// Show telemetry status
    Status,
}

pub fn execute(cli: Cli) -> Result<()> {
    // Initialize logging based on global flags
    crate::logging::init(cli.verbose, cli.debug);

    match cli.command {
        Commands::Init => init::execute(),
        Commands::Set {
            key,
            value,
            env,
            secret,
            expires,
        } => {
            let config = config::find_project_root()?;
            let mut vault = crate::vault::Vault::load(&config)?;

            if let Some(expires_str) = &expires {
                let expires_at = parse_expiry(expires_str)?;
                vault.set_with_expiry(&env, &key, &value, secret, Some(expires_at))?;
            } else {
                vault.set(&env, &key, &value, secret)?;
            }

            let label = if secret { " (secret)" } else { "" };
            let expiry_label = expires
                .map(|e| format!(" (expires: {})", e))
                .unwrap_or_default();
            println!("Set {}{}{} in [{}]", key, label, expiry_label, env);

            crate::audit::log_action(&config, "set", Some(&env), Some(&key))?;
            Ok(())
        }
        Commands::Get { key, env } => {
            let result = get::execute(&key, &env);
            if let Ok(config) = config::find_project_root() {
                let _ = crate::audit::log_action(&config, "get", Some(&env), Some(&key));
            }
            result
        }
        Commands::Rm { key, env } => {
            let config = config::find_project_root()?;
            let mut vault = crate::vault::Vault::load(&config)?;
            vault.remove(&env, &key)?;
            vault.save()?;
            println!("Removed {} from [{}]", key, env);
            crate::audit::log_action(&config, "rm", Some(&env), Some(&key))?;
            Ok(())
        }
        Commands::Ls { env, show } => {
            let config = config::find_project_root()?;
            let vault = crate::vault::Vault::load(&config)?;
            let entries = vault.list(&env)?;

            // Check for expired variables
            let expired = vault.check_expired(&env);
            if !expired.is_empty() {
                use colored::Colorize;
                println!("{}", "Warning: expired variables detected:".yellow());
                for (k, exp) in &expired {
                    println!("  {} (expired {})", k.red(), exp);
                }
                println!();
            }

            if entries.is_empty() {
                println!("No variables set for [{}]", env);
            } else {
                println!("Environment: {}\n", env);
                for (key, value, is_secret) in entries {
                    if show {
                        println!("  {} = {}", key, value);
                    } else if is_secret {
                        println!("  {} = ********", key);
                    } else {
                        let masked = if value.len() > 4 {
                            format!("{}...", &value[..4])
                        } else {
                            "****".to_string()
                        };
                        println!("  {} = {}", key, masked);
                    }
                }
            }
            Ok(())
        }
        Commands::Run { env, cmd } => run::execute(&env, &cmd),
        Commands::Export { env, format } => {
            let result = export::execute(&env, &format);
            if let Ok(config) = config::find_project_root() {
                let _ = crate::audit::log_action(&config, "export", Some(&env), None);
            }
            result
        }
        Commands::Import { file, env, secret } => import::execute(&file, &env, secret),
        Commands::Envs => {
            let config = config::find_project_root()?;
            let vault = crate::vault::Vault::load(&config)?;
            let envs = vault.environments();
            if envs.is_empty() {
                println!("No environments configured.");
            } else {
                println!("Environments:\n");
                for e in envs {
                    let count = vault.list(&e).map(|v| v.len()).unwrap_or(0);
                    println!("  {} ({} variables)", e, count);
                }
            }
            Ok(())
        }
        Commands::Diff { env1, env2, show } => diff::execute(&env1, &env2, show),
        Commands::Lock => {
            let config = config::find_project_root()?;
            let vault = crate::vault::Vault::load(&config)?;
            vault.lock()?;
            println!("Vault locked. .env.vault is safe to commit.");
            crate::audit::log_action(&config, "lock", None, None)?;
            Ok(())
        }
        Commands::Unlock => {
            let config = config::find_project_root()?;
            let mut vault = crate::vault::Vault::load(&config)?;
            vault.unlock()?;
            println!("Vault unlocked.");
            crate::audit::log_action(&config, "unlock", None, None)?;
            Ok(())
        }
        Commands::Key(KeyCommands::Export) => {
            let config = config::find_project_root()?;
            let key = crate::vault::keyring::export_key(&config)?;
            println!("Project key (share securely):\n\n  {}\n", key);
            println!("Team members can import with: envsafe key import <key>");
            Ok(())
        }
        Commands::Key(KeyCommands::Import { key }) => {
            let config = config::find_project_root()?;
            crate::vault::keyring::import_key(&config, &key)?;
            println!("Key imported successfully.");
            Ok(())
        }
        Commands::Validate { env } => {
            let config = config::find_project_root()?;
            let vault = crate::vault::Vault::load(&config)?;
            crate::env::validator::validate(&vault, &env)?;
            Ok(())
        }
        Commands::Hook { action } => match action {
            HookCommands::Install => {
                crate::git::hook::install()?;
                println!("Pre-commit hook installed.");
                Ok(())
            }
            HookCommands::Uninstall => {
                crate::git::hook::uninstall()?;
                println!("Pre-commit hook uninstalled.");
                Ok(())
            }
        },
        Commands::Scan => scan::execute(),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "envsafe", &mut std::io::stdout());
            Ok(())
        }
        Commands::HookShell { shell } => hook_shell::execute(&shell),
        Commands::ManPage => {
            print!(
                r#"ENVSAFE(1)                    User Commands                    ENVSAFE(1)

NAME
       envsafe - Your secrets, encrypted, everywhere.

SYNOPSIS
       envsafe <COMMAND> [OPTIONS]

DESCRIPTION
       EnvSafe is a universal .env and secrets manager. Encrypted local
       vault, git-safe sharing, process injection, environment profiles,
       cloud sync, and more.

COMMANDS
       init                    Initialize envsafe in the current project
       set KEY VALUE           Set an environment variable
       get KEY                 Get an environment variable
       rm KEY                  Remove an environment variable
       ls                      List all environment variables
       run -- CMD              Run command with injected env vars
       export                  Export env vars (shell/dotenv/json/docker/k8s)
       import FILE             Import from a .env file
       envs                    List all environments
       diff ENV1 ENV2          Compare two environments
       lock / unlock           Lock/unlock vault for git sharing
       key export/import       Manage project encryption keys
       validate                Validate against .envsafe.yaml schema
       hook install/uninstall  Manage git pre-commit hook
       scan                    Scan repo for leaked secrets
       completions SHELL       Generate shell completions
       hook-shell SHELL        Print shell hook for auto-injection
       pull/push PROVIDER      Sync with cloud secret managers
       template                Generate .env.example file
       ui                      Open interactive TUI
       rotate-key              Rotate the project encryption key
       audit                   View audit log
       telemetry               Manage telemetry settings
       plugin NAME             Run a plugin
       plugins                 List available plugins

GLOBAL OPTIONS
       --verbose               Enable verbose logging
       --debug                 Enable debug logging

AUTHORS
       envsafe contributors
"#
            );
            Ok(())
        }
        Commands::Pull {
            provider,
            prefix,
            path,
            vault_name,
            env,
        } => {
            let sync_config = crate::sync::SyncConfig {
                env: env.clone(),
                prefix,
                path,
                vault_name,
            };
            let provider = crate::sync::get_provider(&provider)?;
            println!("Pulling from {} ...", provider.name());
            let vars = provider.pull(&sync_config)?;
            if vars.is_empty() {
                println!("No secrets found.");
                return Ok(());
            }
            let project_config = config::find_project_root()?;
            let mut vault = crate::vault::Vault::load(&project_config)?;
            let count = vars.len();
            for (key, value) in &vars {
                vault.set(&env, key, value, true)?;
            }
            println!("Pulled {} secret(s) into [{}].", count, env);
            crate::audit::log_action(&project_config, "pull", Some(&env), None)?;
            Ok(())
        }
        Commands::Push {
            provider,
            prefix,
            path,
            vault_name,
            env,
        } => {
            let sync_config = crate::sync::SyncConfig {
                env: env.clone(),
                prefix,
                path,
                vault_name,
            };
            let project_config = config::find_project_root()?;
            let vault = crate::vault::Vault::load(&project_config)?;
            let vars = vault.get_env_vars(&env)?;
            if vars.is_empty() {
                println!("No variables in [{}] to push.", env);
                return Ok(());
            }
            let provider = crate::sync::get_provider(&provider)?;
            println!(
                "Pushing {} variable(s) to {} ...",
                vars.len(),
                provider.name()
            );
            provider.push(&sync_config, &vars)?;
            println!("Pushed successfully to {}.", provider.name());
            crate::audit::log_action(&project_config, "push", Some(&env), None)?;
            Ok(())
        }
        Commands::Template { env } => template::execute(&env),
        Commands::Tui => crate::tui::run_tui(),
        Commands::RotateKey => {
            let config = config::find_project_root()?;
            crate::vault::rotation::rotate_key(&config)?;
            println!("Key rotated successfully. Old key backed up.");
            crate::audit::log_action(&config, "rotate-key", None, None)?;
            Ok(())
        }
        Commands::Audit { limit } => {
            let config = config::find_project_root()?;
            let entries = crate::audit::read_audit_log(&config)?;
            if entries.is_empty() {
                println!("No audit log entries.");
            } else {
                println!("Audit Log (last {} entries):\n", limit);
                let start = if entries.len() > limit {
                    entries.len() - limit
                } else {
                    0
                };
                for entry in &entries[start..] {
                    println!(
                        "  [{}] {} {} {}",
                        entry.timestamp,
                        entry.action,
                        entry.env.as_deref().unwrap_or(""),
                        entry.key.as_deref().unwrap_or("")
                    );
                }
            }
            Ok(())
        }
        Commands::Telemetry { action } => match action {
            TelemetryCommands::Enable => {
                println!("{}", crate::telemetry::enable());
                Ok(())
            }
            TelemetryCommands::Disable => {
                println!("{}", crate::telemetry::disable());
                Ok(())
            }
            TelemetryCommands::Status => {
                println!("{}", crate::telemetry::status());
                Ok(())
            }
        },
        Commands::Plugin { name, args } => crate::plugin::run(&name, &args),
        Commands::Plugins => {
            let plugins = crate::plugin::discover();
            if plugins.is_empty() {
                println!("No plugins found. Plugins are executables named envsafe-plugin-<name> in your PATH.");
            } else {
                println!("Available plugins:\n");
                for p in plugins {
                    println!("  {} ({})", p.name, p.path.display());
                }
            }
            Ok(())
        }
    }
}

/// Parse an expiry string like "30d", "24h", or "2024-12-31" into an ISO 8601 timestamp
fn parse_expiry(s: &str) -> Result<String> {
    use chrono::{Duration, Utc};

    if let Some(stripped) = s.strip_suffix('d') {
        let days: i64 = stripped
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid expiry: {}", s))?;
        let expires = Utc::now() + Duration::days(days);
        Ok(expires.to_rfc3339())
    } else if let Some(stripped) = s.strip_suffix('h') {
        let hours: i64 = stripped
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid expiry: {}", s))?;
        let expires = Utc::now() + Duration::hours(hours);
        Ok(expires.to_rfc3339())
    } else {
        // Assume ISO date or datetime
        Ok(s.to_string())
    }
}
