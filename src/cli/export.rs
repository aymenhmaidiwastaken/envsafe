use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::config;
use crate::env::parser;

/// Format variables as Docker --env-file format (KEY=VALUE, no quotes, no export)
fn format_docker(vars: &[(String, String)]) -> String {
    let mut output = String::new();
    for (key, value) in vars {
        // Docker env-file format: no quotes, no export prefix
        // Newlines in values are not supported in Docker env-files,
        // so we replace them with literal \n
        let sanitized = value.replace('\n', "\\n");
        output.push_str(&format!("{}={}\n", key, sanitized));
    }
    output
}

/// Format variables as a Kubernetes Secret YAML manifest
fn format_kubernetes(vars: &[(String, String)], env: &str) -> String {
    let mut output = String::new();
    output.push_str("apiVersion: v1\n");
    output.push_str("kind: Secret\n");
    output.push_str("metadata:\n");
    output.push_str(&format!("  name: {}-secrets\n", env));
    output.push_str("  labels:\n");
    output.push_str(&format!("    app.kubernetes.io/env: \"{}\"\n", env));
    output.push_str("    managed-by: envsafe\n");
    output.push_str("type: Opaque\n");
    output.push_str("data:\n");
    for (key, value) in vars {
        let encoded = STANDARD.encode(value.as_bytes());
        output.push_str(&format!("  {}: {}\n", key, encoded));
    }
    output
}

pub fn execute(env: &str, format: &str) -> Result<()> {
    let config = config::find_project_root()?;
    let vault = crate::vault::Vault::load(&config)?;
    let vars = vault.get_env_vars(env)?;

    match format {
        "shell" => {
            for (key, value) in &vars {
                // Shell-safe export format
                let escaped = value.replace('\'', "'\\''");
                println!("export {}='{}'", key, escaped);
            }
        }
        "dotenv" => {
            print!("{}", parser::format_dotenv(&vars));
        }
        "json" => {
            let map: serde_json::Map<String, serde_json::Value> = vars
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            println!("{}", serde_json::to_string_pretty(&map)?);
        }
        "docker" => {
            print!("{}", format_docker(&vars));
        }
        "kubernetes" | "k8s" => {
            print!("{}", format_kubernetes(&vars, env));
        }
        _ => {
            anyhow::bail!(
                "Unknown format '{}'. Use: shell, dotenv, json, docker, kubernetes",
                format
            );
        }
    }

    Ok(())
}
