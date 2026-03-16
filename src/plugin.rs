use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Information about a discovered plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name (without the "envsafe-plugin-" prefix)
    pub name: String,
    /// Full path to the plugin executable
    pub path: PathBuf,
}

/// Plugin executable prefix
const PLUGIN_PREFIX: &str = "envsafe-plugin-";

/// Discover all envsafe plugins on the system PATH.
///
/// Scans each directory in PATH for executables matching `envsafe-plugin-*`.
/// Returns a sorted, deduplicated list of discovered plugins.
pub fn discover() -> Vec<PluginInfo> {
    let path_var = match std::env::var("PATH") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let separator = if cfg!(windows) { ';' } else { ':' };
    let mut plugins: Vec<PluginInfo> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for dir in path_var.split(separator) {
        let dir_path = PathBuf::from(dir);
        if !dir_path.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            // Check for plugin prefix (handle .exe on Windows)
            let base_name = if cfg!(windows) {
                name_str.strip_suffix(".exe").unwrap_or(&name_str)
            } else {
                &name_str
            };

            if let Some(plugin_name) = base_name.strip_prefix(PLUGIN_PREFIX) {
                if plugin_name.is_empty() {
                    continue;
                }

                let plugin_name = plugin_name.to_string();

                // Skip duplicates (first found on PATH wins)
                if seen_names.contains(&plugin_name) {
                    continue;
                }

                let path = entry.path();

                // On Unix, check if the file is executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = path.metadata() {
                        if meta.permissions().mode() & 0o111 == 0 {
                            continue;
                        }
                    }
                }

                seen_names.insert(plugin_name.clone());
                plugins.push(PluginInfo {
                    name: plugin_name,
                    path,
                });
            }
        }
    }

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
}

/// Run a discovered plugin by name, passing arguments and communicating via stdin/stdout JSON.
///
/// The plugin receives arguments as command-line args. Communication protocol:
/// - Plugin reads JSON from stdin (if needed)
/// - Plugin writes JSON results to stdout
/// - Plugin exit code 0 = success, non-zero = failure
pub fn run(name: &str, args: &[String]) -> Result<()> {
    // Find the plugin
    let plugins = discover();
    let plugin = plugins.iter().find(|p| p.name == name).with_context(|| {
        let available: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        if available.is_empty() {
            format!(
                "Plugin '{}' not found. No plugins discovered on PATH.",
                name
            )
        } else {
            format!(
                "Plugin '{}' not found. Available plugins: {}",
                name,
                available.join(", ")
            )
        }
    })?;

    tracing::info!("Running plugin '{}' at {:?}", name, plugin.path);

    let status = Command::new(&plugin.path)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute plugin '{}'", name))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("Plugin '{}' exited with code {}", name, code);
    }

    Ok(())
}
