use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::{Arg, Command};
use toml::Value;

use crate::{
    git,
    toml::{find_key_in_tables, get_toml_content, get_toml_keys},
};
use crate::{git::send_command, plugin::run_plugin};

fn cli() -> Command {
    Command::new("atomic")
        .about("Run custom commands that perform git actions, so you don't have to.")
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List all commands found in atomic.toml")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with_all(["init", "CMD", "template"]),
        )
        .arg(
            Arg::new("init")
                .short('i')
                .long("init")
                .help("Initialize atomic.toml from a template")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with_all(["list", "CMD"]),
        )
        .arg(
            Arg::new("CMD")
                .help("Run a command listed in atomic.toml")
                .index(1)
                .conflicts_with_all(["list", "init", "template"]),
        )
        .arg(
            Arg::new("template")
                .long("template")
                .help("Choose a template: 'rust' or 'default'")
                .value_name("TEMPLATE")
                .requires("init")
                .conflicts_with_all(["list", "CMD"])
                .value_parser(["rust", "default"]),
        )
        .arg(
            Arg::new("PLUGIN")
                .help("Run a plugin defined in [plugin]")
                .long("plugin")
                .value_name("PLUGIN_NAME")
                .conflicts_with_all(["list", "init", "CMD", "template"]),
        )
        .arg_required_else_help(true)
}

/// Main CLI entry point — handles argument parsing and command dispatch
pub fn start_cli() {
    let matches = cli().get_matches();

    // See if the user passed --init to create an atomic.toml file
    let init_selected = matches.get_one::<bool>("init").copied().unwrap_or(false);

    // Check if they passed --list to print all command keys
    let list_selected = matches.get_one::<bool>("list").copied().unwrap_or(false);

    // This grabs a positional command like `atomic build`
    let cmd = matches.get_one::<String>("CMD");

    // This grabs the plugin name if they passed --plugin <name>
    let plugin_name = matches.get_one::<String>("PLUGIN");

    // If --init was used, see if they passed --template (default to "default" if not)
    let template_name = matches
        .get_one::<String>("template")
        .map(String::as_str)
        .unwrap_or("default");

    // We'll use this to track whether a command or plugin actually ran
    let mut command_ran = false;

    // If the user just wants to see available commands, print them and exit
    if list_selected {
        list_keys();
    }
    // If --init was used, try to write a new atomic.toml template to the current folder
    else if init_selected {
        if let Err(err) = start_init(template_name) {
            eprintln!("❌ Failed to initialize atomic.toml: {}", err);
        }
    }
    // If they passed a command like `atomic check`, run it
    else if let Some(cmd) = cmd {
        run_command(cmd, "atomic.toml");
        command_ran = true;
    }
    // If they passed a plugin with --plugin <name>, run that
    else if let Some(plugin) = plugin_name {
        if let Err(err) = run_plugin(plugin, "atomic.toml") {
            eprintln!("❌ Plugin '{}' failed: {}", plugin, err);
        } else {
            command_ran = true;
        }
    }

    // If anything ran successfully, trigger a Git auto-commit
    if command_ran {
        // Use the command name (if available) as context for the commit message
        let cmd_str = cmd.map(|x| x.as_str());

        match git::commit_local_changes(cmd_str) {
            Ok(_) => println!("Auto-commit completed."),
            Err(e) => eprintln!("Auto-commit failed: {}", e),
        }
    }
}

fn list_keys() {
    match get_toml_content("atomic.toml") {
        Some(val) => {
            let keys = get_toml_keys(val);
            if !keys.is_empty() {
                for k in keys {
                    println!("{}", k);
                }
            } else {
                eprintln!("Error reading atomic.toml");
            }
        }
        _ => eprintln!("Error reading atomic.toml"),
    }
}

const RUST_TEMPLATE: &str = include_str!("../template/rust.toml");
const GENERIC_TEMPLATE: &str = include_str!("../template/example.toml");

/// Initializes an `atomic.toml` file using an embedded template.
///
/// - If `atomic.toml` already exists, it will not be overwritten.
/// - Uses the `rust` template if specified; otherwise defaults to a generic template.
///
/// # Arguments
/// * `template_name` - Either `"rust"` or `"default"`
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(io::Error)` if writing the file fails
pub fn start_init(template_name: &str) -> io::Result<()> {
    let atomic_path = Path::new("atomic.toml");

    if atomic_path.exists() {
        println!("⚠️  atomic.toml already exists.");
        print!(
            "Do you want to overwrite it with the '{}' template? [y/N]: ",
            template_name
        );
        io::stdout().flush()?; // flush prompt to terminal

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("❌ Aborted. atomic.toml was not modified.");
            return Ok(());
        }
    }

    // Select which embedded template to use
    let contents = match template_name {
        "rust" => RUST_TEMPLATE,
        _ => GENERIC_TEMPLATE,
    };

    // Write to file
    let mut file = File::create(atomic_path)?;
    file.write_all(contents.as_bytes())?;

    println!("✅ Created atomic.toml using '{}' template.", template_name);
    Ok(())
}

/// Returns the first valid example file found: rust.toml or example.toml
fn find_template_file() -> io::Result<PathBuf> {
    let candidates = [
        Path::new("example/rust.toml"),
        Path::new("example/example.toml"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No template file found in ./example/",
    ))
}

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
    let Some(toml) = load_and_validate_toml(atomic_path) else {
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

/// Loads and validates the `atomic.toml` configuration file.
///
/// - Ensures the file exists
/// - Parses it as TOML
/// - Runs schema validation
///
/// # Arguments
/// * `path` - Path to the `atomic.toml` file
///
/// # Returns
/// * `Some(Value)` if the file was loaded and valid
/// * `None` if any step failed (with error output to stderr)
fn load_and_validate_toml(path: &Path) -> Option<Value> {
    // Check that the file exists
    if !path.exists() {
        eprintln!("❌ atomic.toml not found at {}", path.display());
        return None;
    }

    // Try to parse the TOML file
    let Some(toml) = get_toml_content(path) else {
        eprintln!("❌ Failed to parse '{}'. Is it valid TOML?", path.display());
        return None;
    };

    // Validate the structure of the TOML
    if let Err(errors) = validate_toml_schema(&toml) {
        eprintln!("❌ atomic.toml failed validation:");
        for error in errors {
            eprintln!("  - {}", error);
        }
        return None;
    }

    Some(toml)
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

/// Validates the structure and contents of the `[custom]` section in an `atomic.toml` file.
///
/// This function checks that:
/// - Each `[custom.<name>]` block is either a string (old-style) or a table (new-style).
/// - Each table contains a `command` key.
/// - The `command` key is either a string or an array of strings.
/// - The `before` and `after` keys (if present) are strings.
/// - Any unrecognized keys are flagged as invalid.
/// - Placeholder values like `"command"`, `"todo"`, or `"fixme"` in the `command` field are flagged as likely mistakes.
///
/// # Arguments
///
/// * `toml` - A `toml::Value` parsed from `atomic.toml`.
///
/// # Returns
///
/// * `Ok(())` if the schema is valid.
/// * `Err(Vec<String>)` containing descriptive error messages if validation fails.
///
/// # Example
///
/// ```rust
/// let toml = toml::from_str(r#"
///     [custom.build]
///     command = ["check", "fmt"]
///     before = "echo before"
///     after = "echo after"
/// "#).unwrap();
///
/// assert!(validate_toml_schema(&toml).is_ok());
/// ```
///
/// # Errors
///
/// Returns a list of strings explaining why the validation failed. Common causes include:
/// - Missing `command` key in a `[custom.<name>]` block
/// - `command` is not a string or array of strings
/// - `before` or `after` are not strings
/// - Use of known placeholder values like `"command"` or `"todo"` in the `command` field
///
/// # Related
///
/// This function is typically used immediately after parsing the `atomic.toml` file,
/// before executing any commands, to ensure the configuration is safe and meaningful.
pub fn validate_toml_schema(toml: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Look for the [custom] section in the TOML
    if let Some(custom_section) = toml.get("custom") {
        if let Some(custom_table) = custom_section.as_table() {
            // Loop through each [custom.NAME] block
            for (key, entry) in custom_table {
                match entry {
                    // Old-style: clippy = "cargo clippy" is valid
                    Value::String(_) => {}

                    // New-style: [custom.clippy] with "command", "before", "after"
                    Value::Table(map) => {
                        // Must have a "command" key
                        if !map.contains_key("command") {
                            errors.push(format!("[custom.{}] is missing 'command'", key));
                        }

                        // Validate each key-value pair inside the table
                        for (k, v) in map {
                            let is_valid = match (k.as_str(), v) {
                                // command = "cargo build"
                                ("command", Value::String(_)) => true,

                                // command = ["check", "fmt"]
                                ("command", Value::Array(arr)) => {
                                    arr.iter().all(|item| item.is_str())
                                }

                                // before = "..."
                                ("before", Value::String(_)) => true,

                                // after = "..."
                                ("after", Value::String(_)) => true,

                                // anything else is invalid
                                _ => false,
                            };

                            if !is_valid {
                                errors.push(format!(
                                    "[custom.{}] key '{}' must be a string or array of strings",
                                    key, k
                                ));
                            }
                        }

                        // Extra validation: detect placeholder values like "command" or "todo"
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

                    // Anything else (like command = 123 or command = true) is invalid
                    _ => {
                        errors.push(format!("[custom.{}] must be a string or a table", key));
                    }
                }
            }
            if let Some(plugin_section) = toml.get("plugin") {
                if let Some(plugin_table) = plugin_section.as_table() {
                    for (name, entry) in plugin_table {
                        match entry {
                            Value::Table(map) => {
                                if !map.contains_key("script") {
                                    errors.push(format!(
                                        "[plugin.{}] is missing required 'script'",
                                        name
                                    ));
                                }

                                for (k, v) in map {
                                    match (k.as_str(), v) {
                                        ("script", Value::String(_)) => {}
                                        ("args", Value::Array(arr))
                                            if arr.iter().all(|i| i.is_str()) => {}
                                        ("preferred", Value::String(_)) => {}
                                        ("silent", Value::Boolean(_)) => {} // ✅ allow 'silent' as valid
                                        _ => errors.push(format!(
                                            "[plugin.{}] has invalid key '{}'",
                                            name, k
                                        )),
                                    }
                                }
                            }
                            _ => errors.push(format!("[plugin.{}] must be a table", name)),
                        }
                    }
                } else {
                    errors.push("[plugin] must be a table".into());
                }
            }
        } else {
            errors.push("[custom] must be a table".to_string());
        }
    }

    // Return errors if any were found
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
