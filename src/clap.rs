use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use clap::{arg, Command};
use toml::Value;

use crate::{
    send_command,
    toml::{find_key_in_tables, get_toml_content, get_toml_keys},
};

fn cli() -> Command {
    Command::new("atomic")
        .about("auto local commit while testing without having to think about it")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("custom")
                .about("run command listed in atomic.toml")
                .arg(arg!(<COMMAND> "custom command in atomic.toml")),
        )
        .subcommand(Command::new("init").about("initialize atomic template in project repository"))
        .subcommand(Command::new("build").about("snapshot code then execute build command(s)"))
        .subcommand(Command::new("test").about("shapshot code then perform tests"))
        .subcommand(Command::new("run").about("shapshot code then run program"))
        .subcommand(Command::new("list").about("shapshot code then run program"))
}

pub fn start_cli() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", _sub_matches)) => {
            start_init();
        }
        Some(("custom", sub_matches)) => {
            let cmd = sub_matches
                .get_one::<String>("COMMAND")
                .expect("command is required.");
            run_command(cmd, "atomic.toml");
        }
        Some(("list", _)) => {
            let atomic = get_toml_content("atomic.toml");
            let keys = get_toml_keys(atomic);
            for k in keys {
                println!("{k}");
            }
        }
        Some(("build", _)) | Some(("test", _)) | Some(("run", _)) => {
            if let Some((name, _)) = matches.subcommand() {
                run_command(name, "atomic.toml");
            }
        }

        _ => println!("got {:?}", matches),
    }
}

/// init should simply check to make sure a project folder has a atomic file created in the root.
fn start_init() {
    let atomic = "atomic.toml";

    // if our atomic file does not exist, we create one from a template.
    if let Err(_) = fs::metadata(atomic) {
        if let Ok(_created_file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(atomic)
        {
            // todo: write template to file here
        }
    }
}

fn run_command<P: AsRef<Path>>(cmd: &str, atomic: P) {
    // read in atomic file and parse it out
    let parsed_toml = get_toml_content(atomic);

    let (_, value) = find_key_in_tables(parsed_toml.clone(), cmd).unwrap_or((String::new(), None));

    match value {
        Some(Value::String(s)) => send_command(&s),

        Some(Value::Array(sub_values)) => {
            assert!(!sub_values.is_empty(), "Array of sub-values is empty");

            for v in sub_values {
                // dbg!(&v);
                let inner_value = match find_key_in_tables(parsed_toml.clone(), v.as_str().unwrap())
                {
                    Some((_, Some(inner_value))) => inner_value,
                    _ => v.clone(),
                };
                send_command(&inner_value.as_str().unwrap_or_default());
            }
        }
        _ => {
            // Handle other types of values if necessary
        }
    }
}
