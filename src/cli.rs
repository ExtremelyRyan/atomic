//! cli.rs

use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

use clap::{Arg, Command};

use crate::plugin::run_plugin;
use crate::{
    command::run_command,
    git,
    template::{user_template_path, GENERIC_TEMPLATE, RUST_TEMPLATE},
    toml::list_keys,
};

fn cli() -> Command {
    Command::new("atomic")
        .about("Run custom commands that perform git actions, so you don't have to.")
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List all commands found in atomic.toml")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with_all(["init", "cmd"]),
        )
        .arg(
            Arg::new("init")
                .short('i')
                .long("init")
                .help("Initialize atomic.toml from a template")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with_all(["list", "cmd"]),
        )
        .arg(
            Arg::new("cmd")
                .help("Run a command listed in atomic.toml")
                .index(1)
                .required(false)
                .conflicts_with_all(["list", "init"]),
        )
        .arg(
            Arg::new("plugin")
                .help("Run a plugin defined in [plugin]")
                .short('p')
                .long("plugin")
                .value_name("PLUGIN_NAME")
                .conflicts_with_all(["list", "init", "cmd"]),
        )
.arg(
    Arg::new("squash")
        .help("Squashes local commits and passes commit msg to remote")
        .short('s')
        .long("squash")
        .value_name("COMMIT_MSG")
)
.arg(
    Arg::new("base")
        .help("Base branch to squash to")
        .long("base")
        .value_name("BASE_BRANCH")
        .default_value("main")
        .requires("squash")
)
.arg(
    Arg::new("template")
        .long("template")
        .help("Choose a template")
        .value_name("TEMPLATE")
        .required(false)
)
        .arg_required_else_help(true)
}

/// Main CLI entry point — handles argument parsing and command dispatch
pub fn start_cli() {
    let matches = cli().get_matches();

    // Top-level flags and arguments
    let init_selected = matches.get_one::<bool>("init").copied().unwrap_or(false);
    let template_name = matches
        .get_one::<String>("template")
        .map(String::as_str)
        .unwrap_or("example");

    let list_selected = matches.get_one::<bool>("list").copied().unwrap_or(false);

    let cmd = matches.get_one::<String>("cmd");
    let plugin_name = matches.get_one::<String>("plugin");

    let squash_msg = matches.get_one::<String>("squash");
    let base_branch = matches.get_one::<String>("base").map(String::as_str).unwrap_or("main");

    if let Some(msg) = squash_msg {
        match git::squash_local_commits(base_branch, msg) {
            Ok(_) => println!("Successfully squashed local commits onto {base_branch}."),
            Err(e) => eprintln!("Squash failed: {e}"),
        }
        return;
    }
    // We'll use this to track whether a command or plugin actually ran
    let mut command_ran = false;

    if init_selected {
        if let Err(err) = start_init(template_name) {
            eprintln!("Failed to initialize atomic.toml: {}", err);
        }
        return;
    }

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
