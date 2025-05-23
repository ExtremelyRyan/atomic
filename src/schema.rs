//! schema.rs

use toml::Value;

// -------------
// TOP LEVEL VALIDATOR
// -------------

pub fn validate_toml_schema(toml: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if let Some(custom_section) = toml.get("custom") {
        if let Some(custom_table) = custom_section.as_table() {
            validate_custom_section(custom_table, &mut errors);
        } else {
            errors.push("[custom] must be a table".to_string());
        }
    }

    if let Some(plugin_section) = toml.get("plugin") {
        if let Some(plugin_table) = plugin_section.as_table() {
            validate_plugin_section(plugin_table, &mut errors);
        } else {
            errors.push("[plugin] must be a table".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// -------------
// CUSTOM COMMANDS
// -------------

fn validate_custom_section(custom_table: &toml::value::Table, errors: &mut Vec<String>) {
    for (key, entry) in custom_table {
        match entry {
            Value::String(_) => {} // old-style string command

            Value::Table(map) => {
                validate_custom_entry(key, map, errors);
            }

            _ => errors.push(format!("[custom.{key}] must be a string or a table")),
        }
    }
}

fn validate_custom_entry(key: &str, map: &toml::value::Table, errors: &mut Vec<String>) {
    if !map.contains_key("command") {
        errors.push(format!("[custom.{key}] is missing 'command'"));
    }

    for (k, v) in map {
        let valid = match (k.as_str(), v) {
            ("command" | "desc", Value::Array(arr)) => arr.iter().all(toml::Value::is_str),
            ("command" | "desc", Value::String(_)) => true,
            ("commit", Value::Boolean(_)) | ("before" | "after", Value::String(_)) => true,
            _ => false,
        };

        if !valid {
            errors.push(format!(
                "[custom.{key}] key '{k}' must be a string or array of strings"
            ));
        }
    }

    if let Some(Value::String(cmd)) = map.get("command") {
        let placeholders = ["command", "todo", "fixme", "placeholder"];
        if placeholders.contains(&cmd.to_lowercase().as_str()) {
            errors.push(format!(
                "[custom.{key}] command is set to '{cmd}', which looks like a placeholder"
            ));
        }
    }
}

// -------------
// PLUGINS
// -------------

fn validate_plugin_section(plugin_table: &toml::value::Table, errors: &mut Vec<String>) {
    for (name, entry) in plugin_table {
        match entry {
            Value::Table(map) => {
                validate_plugin_entry(name, map, errors);
            }
            _ => errors.push(format!("[plugin.{name}] must be a table")),
        }
    }
}

fn validate_plugin_entry(name: &str, map: &toml::value::Table, errors: &mut Vec<String>) {
    if !map.contains_key("script") {
        errors.push(format!("[plugin.{name}] is missing required 'script'"));
    }

    for (k, v) in map {
        let valid = match (k.as_str(), v) {
            ("script" | "preferred" | "desc", Value::String(_)) | ("silent", Value::Boolean(_)) => {
                true
            }
            ("args" | "desc", Value::Array(arr)) => arr.iter().all(toml::Value::is_str),
            _ => false,
        };

        if !valid {
            errors.push(format!("[plugin.{name}] has invalid key '{k}'"));
        }
    }
}
