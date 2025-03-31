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

/// Parses a TOML file and returns a vector of all the keys present in it.
/// # Arguments
///
/// * `filename` - A string slice that holds the path to the TOML file.
///
/// # Returns
/// A vector of strings containing all the keys found in the TOML file.
///
/// # Errors
/// This function returns an empty vector if it encounters any errors while reading or parsing the TOML file.
pub fn get_toml_keys(contents: Value) -> Vec<String> {
    let mut keys = Vec::new();
    collect_keys("", &contents, &mut keys);
    keys
}

/// Recursively collects all keys present in a TOML value.
///
/// This function is used internally by `get_toml_keys` to traverse the TOML structure recursively
/// and collect all keys into the provided vector.
///
/// # Arguments
///
fn collect_keys(prefix: &str, value: &Value, keys: &mut Vec<String>) {
    match value {
        Value::Table(table) => {
            for (key, val) in table {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    key.to_string()
                };
                collect_keys(&new_prefix, val, keys);
            }
        }
        _ => {
            keys.push(prefix.to_string());
        }
    }
}

pub fn _table_lookup<'a>(value: &'a Value, table_name: &str, key: &str) -> Option<&'a Value> {
    // Check if the value is a table
    if let Value::Table(table) = value {
        // Check if the specified table exists
        if let Some(Value::Table(inner_table)) = table.get(table_name) {
            // Lookup the key within the table
            return inner_table.get(key);
        }
    }
    None
}
