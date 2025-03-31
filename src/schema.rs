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

            _ => errors.push(format!("[custom.{}] must be a string or a table", key)),
        }
    }
}

fn validate_custom_entry(key: &str, map: &toml::value::Table, errors: &mut Vec<String>) {
    if !map.contains_key("command") {
        errors.push(format!("[custom.{}] is missing 'command'", key));
    }

    for (k, v) in map {
        let valid = match (k.as_str(), v) {
            ("command", Value::String(_)) => true,
            ("command", Value::Array(arr)) => arr.iter().all(|item| item.is_str()),
            ("before", Value::String(_)) => true,
            ("after", Value::String(_)) => true,
            ("desc", Value::String(_)) => true,
            ("desc", Value::Array(arr)) => arr.iter().all(|item| item.is_str()),
            _ => false,
        };

        if !valid {
            errors.push(format!(
                "[custom.{}] key '{}' must be a string or array of strings",
                key, k
            ));
        }
    }

    if let Some(Value::String(cmd)) = map.get("command") {
        let placeholders = ["command", "todo", "fixme", "placeholder"];
        if placeholders.contains(&cmd.to_lowercase().as_str()) {
            errors.push(format!(
                "[custom.{}] command is set to '{}', which looks like a placeholder",
                key, cmd
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
            _ => errors.push(format!("[plugin.{}] must be a table", name)),
        }
    }
}

fn validate_plugin_entry(name: &str, map: &toml::value::Table, errors: &mut Vec<String>) {
    if !map.contains_key("script") {
        errors.push(format!("[plugin.{}] is missing required 'script'", name));
    }

    for (k, v) in map {
        let valid = match (k.as_str(), v) {
            ("script", Value::String(_)) => true,
            ("args", Value::Array(arr)) => arr.iter().all(|i| i.is_str()),
            ("preferred", Value::String(_)) => true,
            ("silent", Value::Boolean(_)) => true,
            ("desc", Value::String(_)) => true,
            ("desc", Value::Array(arr)) => arr.iter().all(|i| i.is_str()),
            _ => false,
        };

        if !valid {
            errors.push(format!("[plugin.{}] has invalid key '{}'", name, k));
        }
    }
}
