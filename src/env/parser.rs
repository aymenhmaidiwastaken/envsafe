use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// Parse a .env file into key-value pairs
pub fn parse_dotenv(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    parse_dotenv_string(&content)
}

/// Parse .env content from a string
pub fn parse_dotenv_string(content: &str) -> Result<BTreeMap<String, String>> {
    let mut vars = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on first '='
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let mut value = line[pos + 1..].trim().to_string();

            // Remove surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }

            // Handle escape sequences in double-quoted values
            if line[pos + 1..].trim().starts_with('"') {
                value = value
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\\\", "\\");
            }

            // Strip inline comments (only for unquoted values)
            if !line[pos + 1..].trim().starts_with('"') && !line[pos + 1..].trim().starts_with('\'')
            {
                if let Some(comment_pos) = value.find(" #") {
                    value = value[..comment_pos].trim_end().to_string();
                }
            }

            if !key.is_empty() {
                vars.insert(key, value);
            }
        }
    }

    Ok(vars)
}

/// Format variables as .env file content
pub fn format_dotenv(vars: &[(String, String)]) -> String {
    let mut output = String::new();
    for (key, value) in vars {
        // Quote values that contain spaces, newlines, or special characters
        if value.contains(' ') || value.contains('\n') || value.contains('#') || value.contains('"')
        {
            let escaped = value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            output.push_str(&format!("{}=\"{}\"\n", key, escaped));
        } else {
            output.push_str(&format!("{}={}\n", key, value));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let input = "KEY=value\nDB_URL=postgres://localhost/db\n";
        let vars = parse_dotenv_string(input).unwrap();
        assert_eq!(vars.get("KEY").unwrap(), "value");
        assert_eq!(vars.get("DB_URL").unwrap(), "postgres://localhost/db");
    }

    #[test]
    fn test_parse_quoted() {
        let input = r#"KEY="hello world"
SINGLE='single quoted'
"#;
        let vars = parse_dotenv_string(input).unwrap();
        assert_eq!(vars.get("KEY").unwrap(), "hello world");
        assert_eq!(vars.get("SINGLE").unwrap(), "single quoted");
    }

    #[test]
    fn test_parse_comments() {
        let input = "# This is a comment\nKEY=value # inline comment\n";
        let vars = parse_dotenv_string(input).unwrap();
        assert_eq!(vars.get("KEY").unwrap(), "value");
        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn test_parse_empty_lines() {
        let input = "\n\nKEY=value\n\n";
        let vars = parse_dotenv_string(input).unwrap();
        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn test_format_dotenv() {
        let vars = vec![
            ("SIMPLE".to_string(), "value".to_string()),
            ("SPACED".to_string(), "hello world".to_string()),
        ];
        let output = format_dotenv(&vars);
        assert!(output.contains("SIMPLE=value"));
        assert!(output.contains("SPACED=\"hello world\""));
    }
}
