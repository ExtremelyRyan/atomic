use std::{
    fs::File,
    io::{self, Write},
    ops::Not,
    path::{Path, PathBuf},
};

use clap::{Arg, Command};
use toml::Value;

use crate::{
    git,
    template::{user_template_path, GENERIC_TEMPLATE, RUST_TEMPLATE},
    toml::{find_key_in_tables, list_keys},
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
                .conflicts_with_all(["init", "CMD"]),
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
                .required(false)
                .conflicts_with_all(["list", "init"]),
        )
        .arg(
            Arg::new("PLUGIN")
                .help("Run a plugin defined in [plugin]")
                .short('p')
                .long("plugin")
                .value_name("PLUGIN_NAME")
                .conflicts_with_all(["list", "init", "CMD"]),
        )
        // .arg(
        //     Arg::new("template")
        //         .long("template")
        //         .help("Choose a template: 'rust', 'default', or a user-defined one")
        //         .value_name("TEMPLATE")
        //         .required(false)
        //         .conflicts_with("CMD"), // Simpler and avoids requires_if issues
        // )
        // .subcommand(
        //     Command::new("template")
        //         .about("Manage custom atomic.toml templates")
        //         .subcommand(
        //             Command::new("save")
        //                 .about("Save the current atomic.toml as a reusable template")
        //                 .arg(
        //                     Arg::new("NAME")
        //                         .help("Name to save the template as")
        //                         .required(true)
        //                         .index(1),
        //                 ),
        //         ),
        // )
        .arg_required_else_help(true)
}

/// Main CLI entry point — handles argument parsing and command dispatch
pub fn start_cli() {
    let matches = cli().get_matches();

    // Top-level flags and arguments
    let init_selected = matches.get_one::<bool>("init").copied().unwrap_or(false);
    let list_selected = matches.get_one::<bool>("list").copied().unwrap_or(false);
    let cmd = matches.get_one::<String>("CMD");
    let plugin_name = matches.get_one::<String>("PLUGIN");

    // We'll use this to track whether a command or plugin actually ran
    let mut command_ran = false;

    // If the user just wants to see available commands, print them and exit
    if list_selected {
        list_keys();
    }
    // If they passed a command like `atomic check`, run it
    else if let Some(cmd) = cmd {
        run_command(cmd, "atomic.toml");
        command_ran = true;
    }
    // If they passed a plugin with --plugin <name>, run that
    else if let Some(plugin) = plugin_name {
        if let Err(err) = run_plugin(plugin, "atomic.toml") {
            eprintln!("Plugin '{}' failed: {}", plugin, err);
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

    let contents = if let Some(user_path) = user_template_path(template_name) {
        if user_path.exists() {
            std::fs::read_to_string(user_path)?
        } else {
            match template_name {
                "rust" => RUST_TEMPLATE.to_string(),
                _ => GENERIC_TEMPLATE.to_string(),
            }
        }
    } else {
        GENERIC_TEMPLATE.to_string()
    };

    // Write to file
    let mut file = File::create(atomic_path)?;
    file.write_all(contents.as_bytes())?;

    println!("✅ Created atomic.toml using '{}' template.", template_name);
    Ok(())
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
