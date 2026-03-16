use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use std::path::Path;

/// Common patterns that indicate leaked secrets
const SECRET_PATTERNS: &[(&str, &str)] = &[
    (
        r#"(?i)aws[_-]?secret[_-]?access[_-]?key\s*=\s*\S+"#,
        "AWS Secret Access Key",
    ),
    (
        r#"(?i)aws[_-]?access[_-]?key[_-]?id\s*=\s*[A-Z0-9]{20}"#,
        "AWS Access Key ID",
    ),
    (r"AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
    (
        r#"(?i)api[_-]?key\s*=\s*['"]?[a-zA-Z0-9_\-]{20,}['"]?"#,
        "API Key",
    ),
    (
        r#"(?i)secret[_-]?key\s*=\s*['"]?[a-zA-Z0-9_\-]{20,}['"]?"#,
        "Secret Key",
    ),
    (r#"(?i)password\s*=\s*['"]?[^\s'"]{8,}['"]?"#, "Password"),
    (r#"(?i)token\s*=\s*['"]?[a-zA-Z0-9_\-.]{20,}['"]?"#, "Token"),
    (r"(?i)private[_-]?key", "Private Key reference"),
    (
        r"-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----",
        "Private Key",
    ),
    (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Access Token"),
    (r"gho_[a-zA-Z0-9]{36}", "GitHub OAuth Token"),
    (r"sk-[a-zA-Z0-9]{32,}", "OpenAI/Stripe Secret Key"),
    (r"sk_live_[a-zA-Z0-9]{24,}", "Stripe Live Secret Key"),
    (r"xox[bporas]-[a-zA-Z0-9\-]+", "Slack Token"),
];

pub fn execute() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let mut findings = Vec::new();

    scan_directory(&current_dir, &mut findings)?;

    if findings.is_empty() {
        println!("{}", "No secrets detected.".green());
    } else {
        println!(
            "{} potential secret(s) found:\n",
            findings.len().to_string().red().bold()
        );
        for (file, line_num, pattern_name, line) in &findings {
            println!("  {}:{}", file.display().to_string().yellow(), line_num);
            println!("    Type: {}", pattern_name);
            println!("    Line: {}", truncate_line(line, 80));
            println!();
        }
        anyhow::bail!(
            "Found {} potential secret leak(s). Review and remove them before committing.",
            findings.len()
        );
    }

    Ok(())
}

fn scan_directory(
    dir: &Path,
    findings: &mut Vec<(std::path::PathBuf, usize, String, String)>,
) -> Result<()> {
    let patterns: Vec<(Regex, &str)> = SECRET_PATTERNS
        .iter()
        .filter_map(|(pat, name)| Regex::new(pat).ok().map(|re| (re, *name)))
        .collect();

    walk_dir(dir, &patterns, findings)?;
    Ok(())
}

fn walk_dir(
    dir: &Path,
    patterns: &[(Regex, &str)],
    findings: &mut Vec<(std::path::PathBuf, usize, String, String)>,
) -> Result<()> {
    let skip_dirs = [
        ".git",
        "node_modules",
        "target",
        ".envsafe",
        "vendor",
        "__pycache__",
        ".venv",
        "venv",
    ];

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if skip_dirs.contains(&name) {
                continue;
            }
        }

        if path.is_dir() {
            walk_dir(&path, patterns, findings)?;
        } else if path.is_file() {
            // Skip binary files, large files, and specific extensions
            if should_scan(&path) {
                scan_file(&path, patterns, findings)?;
            }
        }
    }
    Ok(())
}

fn should_scan(path: &Path) -> bool {
    let skip_extensions = [
        "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "eot", "mp3", "mp4",
        "zip", "tar", "gz", "exe", "dll", "so", "dylib", "enc", "vault",
    ];

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if skip_extensions.contains(&ext.to_lowercase().as_str()) {
            return false;
        }
    }

    // Skip files larger than 1MB
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > 1_000_000 {
            return false;
        }
    }

    true
}

fn scan_file(
    path: &Path,
    patterns: &[(Regex, &str)],
    findings: &mut Vec<(std::path::PathBuf, usize, String, String)>,
) -> Result<()> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Skip unreadable files
    };

    for (line_num, line) in content.lines().enumerate() {
        for (re, name) in patterns {
            if re.is_match(line) {
                findings.push((
                    path.to_path_buf(),
                    line_num + 1,
                    name.to_string(),
                    line.to_string(),
                ));
                break; // One finding per line is enough
            }
        }
    }

    Ok(())
}

fn truncate_line(line: &str, max_len: usize) -> String {
    if line.len() > max_len {
        format!("{}...", &line[..max_len])
    } else {
        line.to_string()
    }
}
