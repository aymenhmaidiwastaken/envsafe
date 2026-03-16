use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::env::parser;
use crate::git;
use crate::vault::keyring;

/// Map a .env filename to an environment name
fn env_file_to_env_name(filename: &str) -> &str {
    match filename {
        ".env" => "dev",
        ".env.local" => "dev",
        ".env.development" => "dev",
        ".env.staging" => "staging",
        ".env.production" => "prod",
        ".env.test" => "test",
        _ => "dev",
    }
}

/// Known .env file patterns to scan for during auto-import
const ENV_FILE_PATTERNS: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.staging",
    ".env.production",
    ".env.test",
];

/// Scan the project root for existing .env files and auto-import them
fn auto_import_env_files(project_root: &Path, config: &ProjectConfig) -> Result<()> {
    let mut found_files: Vec<(String, String)> = Vec::new(); // (filename, env_name)

    for pattern in ENV_FILE_PATTERNS {
        let path = project_root.join(pattern);
        if path.exists() && path.is_file() {
            found_files.push((
                pattern.to_string(),
                env_file_to_env_name(pattern).to_string(),
            ));
        }
    }

    if found_files.is_empty() {
        return Ok(());
    }

    println!();
    println!(
        "{} Found {} .env file(s), auto-importing...",
        "import:".cyan().bold(),
        found_files.len()
    );

    let mut vault = crate::vault::Vault::load(config)?;

    for (filename, env_name) in &found_files {
        let path = project_root.join(filename);
        match parser::parse_dotenv(&path) {
            Ok(vars) => {
                if vars.is_empty() {
                    println!(
                        "  {} {} (empty, skipped)",
                        filename.dimmed(),
                        format!("-> [{}]", env_name).dimmed()
                    );
                    continue;
                }
                let count = vars.len();
                for (key, value) in &vars {
                    vault.set(env_name, key, value, false)?;
                }
                println!(
                    "  {} {} {} variable(s)",
                    filename,
                    format!("-> [{}]", env_name).green(),
                    count
                );
            }
            Err(e) => {
                println!("  {} {} (failed: {})", filename, "skipped".yellow(), e);
            }
        }
    }

    println!();
    println!(
        "{}",
        "Tip: You can now delete your .env files. Secrets are stored in the vault.".dimmed()
    );

    Ok(())
}

pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let envsafe_dir = project_root.join(".envsafe");

    if envsafe_dir.exists() {
        anyhow::bail!("Already initialized. .envsafe directory exists.");
    }

    // Create project config
    let config = ProjectConfig::new(project_root.clone());
    config.save()?;

    // Generate master key (stored globally, NOT in project)
    keyring::create_key(&config)?;

    // Create initial vault
    let vault = crate::vault::Vault::load(&config)?;
    vault.save()?;

    // Auto-add .env patterns to .gitignore
    if project_root.join(".git").exists() {
        git::ignore::add_patterns(&project_root)?;
    }

    println!("{}", "envsafe initialized!".green().bold());
    println!();
    println!("  Project ID: {}", config.project_id);
    println!("  Vault: .envsafe/vault.enc");
    println!("  Key stored in: ~/.config/envsafe/keys/");

    // Auto-import existing .env files
    auto_import_env_files(&project_root, &config)?;

    println!();
    println!("Get started:");
    println!("  envsafe set DATABASE_URL \"postgres://localhost/mydb\"");
    println!("  envsafe run -- npm start");

    Ok(())
}
