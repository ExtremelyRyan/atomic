//! commmand.rs

use std::path::Path;

use toml::Value;

use crate::{git::send_command, toml::find_key_in_tables};

/// Entry point to run a named command from the `atomic.toml` configuration.
///
/// This function:
/// - Loads and validates the `atomic.toml` file
/// - Finds the requested command by name
/// - Executes it using hook/chain resolution logic
///
/// # Arguments
/// * `cmd` - The name of the command to run (e.g. "clippy", "chain")
/// * `atomic` - A path to the `atomic.toml` file or project directory
pub fn run_command<P: AsRef<Path>>(cmd: &str, atomic: P) {
    let atomic_path = atomic.as_ref();

    // Load and validate atomic.toml from the given path
    let Some(toml) = crate::toml::load_and_validate_toml(atomic_path) else {
        return; // Exit early if the file is missing or invalid
    };

    // Attempt to find the command in the parsed TOML tables
    let Some((_, value)) = find_key_in_tables(toml.clone(), cmd) else {
        eprintln!("Command '{}' not found in atomic.toml", cmd);
        return;
    };

    // Dispatch to execution logic with the resolved value
    execute_resolved_command(value.as_ref(), cmd, &toml, atomic_path);
}

/// Executes a resolved command from the TOML configuration.
///
/// This function handles:
/// - Raw string commands
/// - Chains (arrays of subcommands)
/// - Hook-based tables with `before`, `command`, `after`
///
/// # Arguments
/// * `value` - The TOML value associated with the command
/// * `cmd_name` - The original command name (for logging/errors)
/// * `toml` - The full parsed TOML for resolving nested commands
/// * `toml_path` - Path to the atomic.toml file
fn execute_resolved_command(value: Option<&Value>, cmd_name: &str, toml: &Value, toml_path: &Path) {
    match value {
        // Simple shell command
        Some(Value::String(s)) => {
            println!("Resolving subcommand: {}", s);
            send_command(s);
        }

        // Array of subcommands or raw strings
        Some(Value::Array(sub_commands)) => {
            for val in sub_commands {
                if let Some(sub_cmd) = val.as_str() {
                    resolve_and_run_subcommand(sub_cmd, toml, toml_path);
                }
            }
        }

        // Table with possible hooks or nested chaining
        Some(Value::Table(table)) => {
            // If the command is an array, treat it as a chained sequence
            if let Some(Value::Array(chain)) = table.get("command") {
                for val in chain {
                    if let Some(sub_cmd) = val.as_str() {
                        resolve_and_run_subcommand(sub_cmd, toml, toml_path);
                    }
                }
                return; // Prevent falling through to hook logic
            }

            // Otherwise run the table as a hook-based command
            run_table_command(table, cmd_name);
        }

        // Unsupported or invalid TOML structure
        _ => {
            eprintln!("Unsupported command format for '{}'", cmd_name);
        }
    }
}

/// Resolves a subcommand by name, then executes it.
///
/// If the subcommand matches an entry in the TOML config, it is executed recursively.
/// If not, it is treated as a raw shell command.
///
/// # Arguments
/// * `sub_cmd` - The subcommand name or shell command
/// * `toml` - The full parsed TOML config
/// * `atomic_path` - Path to the `atomic.toml` file for recursion
fn resolve_and_run_subcommand(sub_cmd: &str, toml: &Value, atomic_path: &Path) {
    match find_key_in_tables(toml.clone(), sub_cmd) {
        Some((_, Some(Value::String(_) | Value::Array(_) | Value::Table(_)))) => {
            // It's a declared custom command; run it recursively
            run_command(sub_cmd, atomic_path);
        }
        _ => {
            // Fall back to executing it as a raw shell string
            send_command(sub_cmd);
        }
    }
}

/// Executes a single `[custom.command]` table with optional hooks.
///
/// Runs the command in this order:
/// 1. `before` (if defined)
/// 2. `command` (required, must be a string)
/// 3. `after` (if defined)
///
/// # Arguments
/// * `table` - A reference to the TOML table for this command
/// * `label` - The name of the command (for logging and error messages)
fn run_table_command(table: &toml::value::Table, label: &str) {
    // Optional "before" hook
    if let Some(before) = table.get("before").and_then(|v| v.as_str()) {
        send_command(before);
    }

    // Required "command" key
    if let Some(main) = table.get("command").and_then(|v| v.as_str()) {
        send_command(main);
    } else {
        eprintln!("Missing 'command' in table '{}'", label);
    }

    // Optional "after" hook
    if let Some(after) = table.get("after").and_then(|v| v.as_str()) {
        send_command(after);
    }
}
