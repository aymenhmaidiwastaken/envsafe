use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const PROJECT_DIR: &str = ".envsafe";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project_id: String,
    pub project_root: PathBuf,
    pub created_at: String,
}

impl ProjectConfig {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_id: uuid::Uuid::new_v4().to_string(),
            project_root,
            created_at: chrono_now(),
        }
    }

    pub fn envsafe_dir(&self) -> PathBuf {
        self.project_root.join(PROJECT_DIR)
    }

    pub fn vault_path(&self) -> PathBuf {
        self.envsafe_dir().join("vault.enc")
    }

    pub fn config_path(&self) -> PathBuf {
        self.envsafe_dir().join(CONFIG_FILE)
    }

    pub fn save(&self) -> Result<()> {
        let dir = self.envsafe_dir();
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(self.config_path(), json)?;
        Ok(())
    }

    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(PROJECT_DIR).join(CONFIG_FILE);
        let data = std::fs::read_to_string(&config_path)
            .with_context(|| "Not an envsafe project. Run `envsafe init` first.")?;
        let config: Self = serde_json::from_str(&data)?;
        Ok(config)
    }
}

/// Walk up from current dir to find the project root containing .envsafe/
pub fn find_project_root() -> Result<ProjectConfig> {
    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(PROJECT_DIR).join(CONFIG_FILE);
        if candidate.exists() {
            return ProjectConfig::load(&current);
        }
        if !current.pop() {
            anyhow::bail!("Not an envsafe project. Run `envsafe init` first.");
        }
    }
}

/// Global config directory (~/.config/envsafe/)
pub fn global_config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("envsafe");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Keys directory (~/.config/envsafe/keys/)
pub fn keys_dir() -> Result<PathBuf> {
    let dir = global_config_dir()?.join("keys");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn chrono_now() -> String {
    // Simple ISO-8601 timestamp without chrono dependency
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
