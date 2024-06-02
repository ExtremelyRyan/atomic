use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use clap::{arg, Command};
use toml::Value;

use crate::git::send_command;
use crate::toml::{find_key_in_tables, get_toml_content, get_toml_keys};

fn cli() -> Command {
    Command::new("atomic")
        .about("run custom commands that perform git actions, so you dont have to.")
        .arg(arg!(-l --list "list all commands found in project atomic.toml").exclusive(true))
        .arg(arg!(-i --init "initialize atomic template in project repository").exclusive(true))
        .arg(arg!([CMD] "run command listed in projects atomic.toml").exclusive(true))
        .arg_required_else_help(true)
}

pub fn start_cli() {
    let matches = cli().get_matches();

    if matches
        .get_one::<bool>("list")
        .is_some_and(|b| b.to_owned() == true)
    {
        let atomic = get_toml_content("atomic.toml");
        let keys = get_toml_keys(atomic);
        for k in keys {
            println!("{k}");
        }
    } else if matches
        .get_one::<bool>("init")
        .is_some_and(|b| b.to_owned() == true)
    {
        start_init();
    } else if let Some(cmd) = matches.get_one::<String>("CMD") {
        dbg!("cmd");
        run_command(cmd, "atomic.toml");
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
