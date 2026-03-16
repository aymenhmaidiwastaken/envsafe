use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Telemetry is DISABLED by default. Users must explicitly opt in
/// with `envsafe telemetry enable` before any data is collected.
///
/// All telemetry is stored locally in ~/.config/envsafe/telemetry.json.
/// No network calls are made. The data collected is minimal:
/// - Command name (e.g., "init", "set", "export")
/// - Operating system and architecture
/// - envsafe version

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub command: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TelemetryData {
    pub config: TelemetryConfig,
    pub events: Vec<TelemetryEvent>,
}

/// Returns the path to the telemetry data file: ~/.config/envsafe/telemetry.json
fn telemetry_path() -> Option<PathBuf> {
    dirs::config_dir().map(|config| config.join("envsafe").join("telemetry.json"))
}

/// Loads telemetry data from disk. Returns default (disabled) if file doesn't exist.
fn load_data() -> TelemetryData {
    let Some(path) = telemetry_path() else {
        return TelemetryData::default();
    };

    if !path.exists() {
        return TelemetryData::default();
    }

    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => TelemetryData::default(),
    }
}

/// Saves telemetry data to disk.
fn save_data(data: &TelemetryData) -> Result<(), String> {
    let path =
        telemetry_path().ok_or_else(|| "Could not determine config directory".to_string())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let json =
        serde_json::to_string_pretty(data).map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write telemetry file: {}", e))?;

    Ok(())
}

/// Check if telemetry is enabled.
pub fn init() -> bool {
    let data = load_data();
    data.config.enabled
}

/// Record a telemetry event (only if enabled).
pub fn record(command: &str) {
    let mut data = load_data();

    if !data.config.enabled {
        return; // Silently skip if telemetry is disabled
    }

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let event = TelemetryEvent {
        command: command.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
    };

    data.events.push(event);

    // Best-effort save, don't fail the command if telemetry write fails
    let _ = save_data(&data);
}

/// Enable telemetry collection. Returns a user-facing message.
pub fn enable() -> String {
    let mut data = load_data();
    data.config.enabled = true;

    match save_data(&data) {
        Ok(_) => {
            let path = telemetry_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            format!(
                "Telemetry enabled. Anonymous usage data will be stored locally at:\n  {}\n\n\
                 Data collected: command name, OS, architecture, envsafe version.\n\
                 No data is sent over the network.",
                path
            )
        }
        Err(e) => format!("Failed to enable telemetry: {}", e),
    }
}

/// Disable telemetry collection. Returns a user-facing message.
pub fn disable() -> String {
    let mut data = load_data();
    data.config.enabled = false;
    data.events.clear(); // Clear any previously collected data

    match save_data(&data) {
        Ok(_) => "Telemetry disabled. All previously collected data has been cleared.".to_string(),
        Err(e) => format!("Failed to disable telemetry: {}", e),
    }
}

/// Get the current telemetry status. Returns a user-facing message.
pub fn status() -> String {
    let data = load_data();
    let state = if data.config.enabled {
        "enabled"
    } else {
        "disabled"
    };

    let path = telemetry_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let event_count = data.events.len();

    format!(
        "Telemetry: {}\nData file: {}\nEvents recorded: {}\n\n\
         Data collected (when enabled): command name, OS, architecture, envsafe version.\n\
         All data is stored locally. No data is sent over the network.\n\n\
         Use 'envsafe telemetry enable' to opt in.\n\
         Use 'envsafe telemetry disable' to opt out and clear data.",
        state, path, event_count
    )
}
