use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::config;

/// Mask a value, showing only the first 4 characters followed by "..."
fn mask_value(value: &str) -> String {
    if value.len() > 4 {
        format!("{}...", &value[..4])
    } else {
        format!("{}...", value)
    }
}

/// Enhanced side-by-side diff between two environments
pub fn execute(env1: &str, env2: &str, show: bool) -> Result<()> {
    let project_config = config::find_project_root()?;
    let vault = crate::vault::Vault::load(&project_config)?;

    let entries1 = vault.list(env1)?;
    let entries2 = vault.list(env2)?;

    let keys1: BTreeSet<String> = entries1.iter().map(|(k, _, _)| k.clone()).collect();
    let keys2: BTreeSet<String> = entries2.iter().map(|(k, _, _)| k.clone()).collect();
    let all_keys: BTreeSet<String> = keys1.union(&keys2).cloned().collect();

    if all_keys.is_empty() {
        println!("Both environments are empty.");
        return Ok(());
    }

    // Build lookup maps
    let map1: std::collections::HashMap<&str, (&str, bool)> = entries1
        .iter()
        .map(|(k, v, s)| (k.as_str(), (v.as_str(), *s)))
        .collect();
    let map2: std::collections::HashMap<&str, (&str, bool)> = entries2
        .iter()
        .map(|(k, v, s)| (k.as_str(), (v.as_str(), *s)))
        .collect();

    // Calculate column widths
    let max_key_len = all_keys.iter().map(|k| k.len()).max().unwrap_or(3).max(3);
    let col_width = 16;

    // Print header
    println!();
    println!(
        "  {:<width$} {:<col$} {:<col$} {}",
        "KEY".bold(),
        format!("[{}]", env1).cyan().bold(),
        format!("[{}]", env2).cyan().bold(),
        "STATUS".bold(),
        width = max_key_len,
        col = col_width,
    );

    // Print separator
    let total_width = max_key_len + col_width * 2 + 20;
    println!("  {}", "\u{2500}".repeat(total_width).dimmed());

    let mut same_count = 0;
    let mut diff_count = 0;
    let mut only1_count = 0;
    let mut only2_count = 0;

    for key in &all_keys {
        let v1 = map1.get(key.as_str());
        let v2 = map2.get(key.as_str());

        let (val1_display, val2_display, status, status_color) = match (v1, v2) {
            (Some((val, _secret)), None) => {
                only1_count += 1;
                let display = if show {
                    val.to_string()
                } else {
                    mask_value(val)
                };
                (
                    display,
                    "(not set)".to_string(),
                    format!("(only in {})", env1),
                    "red",
                )
            }
            (None, Some((val, _secret))) => {
                only2_count += 1;
                let display = if show {
                    val.to_string()
                } else {
                    mask_value(val)
                };
                (
                    "(not set)".to_string(),
                    display,
                    format!("(only in {})", env2),
                    "green",
                )
            }
            (Some((val1, _s1)), Some((val2, _s2))) => {
                if *val1 == *val2 {
                    same_count += 1;
                    let d1 = if show {
                        val1.to_string()
                    } else {
                        mask_value(val1)
                    };
                    let d2 = if show {
                        val2.to_string()
                    } else {
                        mask_value(val2)
                    };
                    (d1, d2, "(same)".to_string(), "dimmed")
                } else {
                    diff_count += 1;
                    let d1 = if show {
                        val1.to_string()
                    } else {
                        mask_value(val1)
                    };
                    let d2 = if show {
                        val2.to_string()
                    } else {
                        mask_value(val2)
                    };
                    (d1, d2, "(differs)".to_string(), "yellow")
                }
            }
            (None, None) => unreachable!(),
        };

        // Truncate display values to fit columns
        let v1_trunc = if val1_display.len() > col_width - 2 {
            format!("{}...", &val1_display[..col_width - 5])
        } else {
            val1_display
        };
        let v2_trunc = if val2_display.len() > col_width - 2 {
            format!("{}...", &val2_display[..col_width - 5])
        } else {
            val2_display
        };

        let colored_status = match status_color {
            "red" => status.red().to_string(),
            "green" => status.green().to_string(),
            "yellow" => status.yellow().to_string(),
            "dimmed" => status.dimmed().to_string(),
            _ => status,
        };

        let colored_key = match status_color {
            "red" => key.red().to_string(),
            "green" => key.green().to_string(),
            "yellow" => key.yellow().to_string(),
            _ => key.to_string(),
        };

        println!(
            "  {:<width$} {:<col$} {:<col$} {}",
            colored_key,
            v1_trunc,
            v2_trunc,
            colored_status,
            width = max_key_len,
            col = col_width,
        );
    }

    // Summary
    println!();
    println!(
        "  Summary: {} same, {} differ, {} only in [{}], {} only in [{}]",
        same_count.to_string().dimmed(),
        diff_count.to_string().yellow(),
        only1_count.to_string().red(),
        env1,
        only2_count.to_string().green(),
        env2,
    );
    println!();

    Ok(())
}
