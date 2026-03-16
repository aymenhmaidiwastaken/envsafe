use anyhow::Result;

const HOOK_CONTENT: &str = r#"#!/bin/sh
# envsafe pre-commit hook — prevents committing .env files and scans for secrets

# Check for .env files being committed
env_files=$(git diff --cached --name-only | grep -E '^\\.env($|\\.)' | grep -v '\\.env\\.vault$' | grep -v '\\.env\\.example$')

if [ -n "$env_files" ]; then
    echo "envsafe: Blocked commit — .env files detected in staging:"
    echo "$env_files" | sed 's/^/  /'
    echo ""
    echo "Remove them with: git reset HEAD <file>"
    echo "These files should not be committed. Use 'envsafe lock' to create a git-safe vault."
    exit 1
fi

# Run secret scan if envsafe is available
if command -v envsafe >/dev/null 2>&1; then
    envsafe scan
    if [ $? -ne 0 ]; then
        echo "envsafe: Secret scan failed. Fix issues above before committing."
        exit 1
    fi
fi
"#;

/// Install the pre-commit hook
pub fn install() -> Result<()> {
    let hooks_dir = find_hooks_dir()?;
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("pre-commit");

    if hook_path.exists() {
        let existing = std::fs::read_to_string(&hook_path)?;
        if existing.contains("envsafe") {
            anyhow::bail!("envsafe pre-commit hook is already installed");
        }
        // Append to existing hook
        let mut content = existing;
        content.push_str("\n\n");
        content.push_str(HOOK_CONTENT);
        std::fs::write(&hook_path, content)?;
    } else {
        std::fs::write(&hook_path, HOOK_CONTENT)?;
    }

    // Make executable (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Uninstall the pre-commit hook
pub fn uninstall() -> Result<()> {
    let hooks_dir = find_hooks_dir()?;
    let hook_path = hooks_dir.join("pre-commit");

    if !hook_path.exists() {
        anyhow::bail!("No pre-commit hook found");
    }

    let content = std::fs::read_to_string(&hook_path)?;
    if !content.contains("envsafe") {
        anyhow::bail!("Pre-commit hook doesn't contain envsafe hook");
    }

    // If the hook is entirely ours, remove it
    if content.trim() == HOOK_CONTENT.trim() {
        std::fs::remove_file(&hook_path)?;
    } else {
        // Remove just our section
        let cleaned = content.replace(HOOK_CONTENT, "");
        std::fs::write(&hook_path, cleaned)?;
    }

    Ok(())
}

fn find_hooks_dir() -> Result<std::path::PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Ok(git_dir.join("hooks"));
        }
        if !current.pop() {
            anyhow::bail!("Not a git repository");
        }
    }
}
