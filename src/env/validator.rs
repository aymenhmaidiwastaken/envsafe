use crate::vault::Vault;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Schema {
    required: Vec<VarSchema>,
}

#[derive(Debug, Deserialize)]
struct VarSchema {
    name: String,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    default: Option<serde_yaml::Value>,
}

/// Validate environment variables against .envsafe.yaml schema
pub fn validate(vault: &Vault, env: &str) -> Result<()> {
    let schema_path = find_schema()?;
    let content = std::fs::read_to_string(&schema_path).context("Failed to read .envsafe.yaml")?;
    let schema: Schema = serde_yaml::from_str(&content).context("Invalid .envsafe.yaml format")?;

    let vars = vault.get_env_vars(env)?;
    let var_map: std::collections::HashMap<_, _> = vars.into_iter().collect();

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for rule in &schema.required {
        match var_map.get(&rule.name) {
            None => {
                if rule.default.is_some() {
                    warnings.push(format!("{}: not set, will use default", rule.name));
                } else {
                    errors.push(format!(
                        "{}: required but not set{}",
                        rule.name,
                        rule.description
                            .as_ref()
                            .map(|d| format!(" ({})", d))
                            .unwrap_or_default()
                    ));
                }
            }
            Some(value) => {
                // Check pattern
                if let Some(pattern) = &rule.pattern {
                    let re = regex::Regex::new(pattern)
                        .with_context(|| format!("Invalid pattern for {}", rule.name))?;
                    if !re.is_match(value) {
                        errors.push(format!(
                            "{}: value doesn't match pattern '{}'",
                            rule.name, pattern
                        ));
                    }
                }

                // Check type
                if let Some(type_name) = &rule.r#type {
                    match type_name.as_str() {
                        "integer" => {
                            if value.parse::<i64>().is_err() {
                                errors.push(format!(
                                    "{}: expected integer, got '{}'",
                                    rule.name, value
                                ));
                            }
                        }
                        "boolean" => {
                            if !matches!(
                                value.to_lowercase().as_str(),
                                "true" | "false" | "1" | "0"
                            ) {
                                errors.push(format!(
                                    "{}: expected boolean, got '{}'",
                                    rule.name, value
                                ));
                            }
                        }
                        "url" => {
                            if !value.starts_with("http://") && !value.starts_with("https://") {
                                errors
                                    .push(format!("{}: expected URL, got '{}'", rule.name, value));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Print results
    if errors.is_empty() && warnings.is_empty() {
        println!("{}", "All validations passed!".green());
        return Ok(());
    }

    for w in &warnings {
        println!("  {} {}", "warning:".yellow(), w);
    }

    if !errors.is_empty() {
        for e in &errors {
            println!("  {} {}", "error:".red(), e);
        }
        anyhow::bail!("Validation failed with {} error(s)", errors.len());
    }

    Ok(())
}

fn find_schema() -> Result<std::path::PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(".envsafe.yaml");
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate = current.join(".envsafe.yml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !current.pop() {
            anyhow::bail!("No .envsafe.yaml schema file found");
        }
    }
}
