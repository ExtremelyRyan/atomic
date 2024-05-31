use std::{fs::read_to_string, path::Path};
use toml::Value;

pub fn find_key_in_tables(parsed_toml: Value, key: &str) -> Option<(String, Option<Value>)> {
    if let Some(table) = parsed_toml.as_table() {
        for (k, v) in table {
            if let Some(inner_table) = v.as_table() {
                if inner_table.contains_key(key) {
                    return Some((k.clone(), inner_table.get(key).cloned()));
                }
            }
        }
    }
    None
}

pub fn get_toml_content<P>(atomic: P) -> Value
where
    P: AsRef<Path>,
{
    let contents = read_to_string(atomic.as_ref()).expect("Unable to read atomic file");
    toml::from_str(&contents).expect("Unable to read atomic file")
}

#[allow(dead_code)]
pub fn parse_toml_value(parsed_toml: Value) {
    // Extract items from the TOML value
    match parsed_toml {
        Value::Table(table) => {
            // Iterate over key-value pairs in the table
            for (_key, value) in &table {
                println!("{:#?}", value.as_array());
            }
        }
        Value::Array(array) => {
            // Iterate over items in the array
            for item in array {
                print!(" {}, ", item.to_string());
            }
        }
        Value::String(string_val) => {
            println!("{}", string_val);
        }
        Value::Integer(int_val) => {
            println!("Integer value: {}", int_val);
        }
        Value::Float(float_val) => {
            println!("Float value: {}", float_val);
        }
        Value::Boolean(bool_val) => {
            println!("Boolean value: {}", bool_val);
        }
        _ => {
            println!("Other type of value: {:?}", parsed_toml);
        }
    }
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
                    format!("{}", key)
                };
                collect_keys(&new_prefix, val, keys);
            }
        }
        _ => {
            keys.push(prefix.to_string());
        }
    }
}

pub fn table_lookup<'a>(value: &'a Value, table_name: &str, key: &str) -> Option<&'a Value> {
    // Check if the value is a table
    if let Value::Table(table) = value {
        // Check if the specified table exists
        if let Some(table_value) = table.get(table_name) {
            // Check if the table value is indeed a table
            if let Value::Table(inner_table) = table_value {
                // Lookup the key within the table
                return inner_table.get(key);
            }
        }
    }
    None
}
