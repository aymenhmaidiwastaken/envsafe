use anyhow::Result;
use std::path::Path;

const ENVSAFE_PATTERNS: &[&str] = &[
    "",
    "# envsafe - secret management",
    ".env",
    ".env.*",
    "!.env.vault",
    "!.env.example",
    ".envsafe/vault.enc",
    ".env.keys",
];

/// Add envsafe patterns to .gitignore
pub fn add_patterns(project_root: &Path) -> Result<()> {
    let gitignore_path = project_root.join(".gitignore");

    let existing = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // Check if we've already added our patterns
    if existing.contains("# envsafe - secret management") {
        return Ok(());
    }

    let mut content = existing;
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }

    for pattern in ENVSAFE_PATTERNS {
        content.push_str(pattern);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, content)?;
    Ok(())
}
