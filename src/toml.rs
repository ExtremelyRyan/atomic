use std::{fs::read_to_string, path::Path};
use toml::Value;

pub fn find_key_in_tables(parsed_toml: Value, key: &str) -> Option<(String, Option<Value>)> {
    // Directly check if the key exists at the root level
    if let Some(table) = parsed_toml.as_table() {
        if table.contains_key(key) {
            return Some((key.to_string(), table.get(key).cloned()));
        }

        // Search nested tables, e.g., "custom"
        for (k, v) in table {
            if let Value::Table(inner_table) = v {
                if inner_table.contains_key(key) {
                    return Some((k.clone(), inner_table.get(key).cloned()));
                }
            }
        }

        // Explicitly check "custom" table if it exists
        if let Some(Value::Table(custom_table)) = table.get("custom") {
            if let Some(value) = custom_table.get(key) {
                return Some(("custom".to_string(), Some(value.clone())));
            }
        }
    }

    None
}

pub fn get_toml_content<P>(atomic: P) -> Option<Value>
where
    P: AsRef<Path>,
{
    let contents = read_to_string(atomic.as_ref()).expect("Unable to read atomic file");
    toml::from_str(&contents).ok()
}

/// Prints all user-accessible command keys defined in atomic.toml,
/// grouped by section ([default], [custom], [plugin]).
/// Also prints descriptions if the user defined a `desc` field.
pub fn list_keys() {
    match get_toml_content("atomic.toml") {
        Some(toml) => {
            let mut found = false;

            // --- [default] section ---
            // These are simple key-value pairs like build = "cargo build"
            if let Some(defaults) = toml.get("default").and_then(|v| v.as_table()) {
                println!("[default]");
                for key in defaults.keys() {
                    println!("  - {}", key); // Just print the command name (no description support here yet)
                }
                found = true;
            }

            // --- [custom] section ---
            // These can be either simple string/array commands or full tables with hooks and descriptions
            if let Some(custom) = toml.get("custom").and_then(|v| v.as_table()) {
                println!("[custom]");
                for (key, val) in custom.iter() {
                    match val {
                        // If the command is a full table, try to extract and show the description
                        toml::Value::Table(inner) => {
                            let desc = inner
                                .get("desc")
                                .and_then(|d| d.as_str())
                                .unwrap_or("<no description>");
                            println!("  - {:<12} {}", key, desc);
                        }

                        // If the command is a raw string or array (e.g., build = "cargo build")
                        toml::Value::String(_) | toml::Value::Array(_) => {
                            println!("  - {:<12} <no description>", key);
                        }

                        // Ignore unexpected formats
                        _ => {}
                    }
                }
                found = true;
            }

            // --- [plugin] section ---
            // Plugins are scripts, often run with --plugin <name>. We show their description if available.
            if let Some(plugins) = toml.get("plugin").and_then(|v| v.as_table()) {
                println!("[plugin]");
                for (key, val) in plugins.iter() {
                    let desc = val
                        .as_table()
                        .and_then(|t| t.get("desc"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("<no description>");
                    println!("  - {:<12} {}", key, desc);
                }
                found = true;
            }

            // If no valid sections were found at all
            if !found {
                eprintln!("No commands found in atomic.toml.");
            }
        }

        // Couldn’t read or parse the atomic.toml file
        None => {
            eprintln!("Failed to read atomic.toml.");
        }
    }
}