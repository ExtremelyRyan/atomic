use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use clap::{arg, Command};
use toml::Value;

use crate::send_command;

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

// fn push_args() -> Vec<clap::Arg> {
//     vec![arg!(-m --message <MESSAGE>)]
// }

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
            parse_toml_value(atomic);
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

// fn run_command<P: AsRef<Path>>(cmd: &String, atomic: P) {
//     // read in atomic file and parse it out

//     // Deserialize the TOML data into a generic data structure
//     let parsed_toml = get_toml_content(atomic);

//     let (_, value) = find_key_in_tables(parsed_toml.clone(), cmd).unwrap_or((String::new(), None));

//     // todo: find better output method than this

//     let s = value
//         .clone()
//         .and_then(|v| Some(v.clone().as_str().unwrap_or("").to_string()))
//         .unwrap()
//         .clone();

//     // is a string, not array | table
//     if !s.is_empty() {
//         send_command(&s);
//         return;
//     }

//     // blank default value so it is safe to unwrap
//     let binding = value.unwrap_or_else(|| Value::Array(vec![]));

//     let sub_values = binding.as_array().unwrap();

//     debug_assert!(!sub_values.is_empty());

//     sub_values.into_iter().for_each(|v| {
//         // if we find the value is another key within the file - run that keys commands
//         if let Some((_inner_table_name, Some(inner_value))) =
//             find_key_in_tables(parsed_toml.clone(), v.as_str().unwrap())
//         {
//             // println!("inner_inner value: {}", inner_value);
//             send_command(&inner_value.as_str().unwrap_or_default());
//         } else {
//             // otherwise run the value directly
//             send_command(v.as_str().unwrap_or_default());
//         }
//     });
// }

fn find_key_in_tables(parsed_toml: Value, key: &str) -> Option<(String, Option<Value>)> {
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

fn get_toml_content<P>(atomic: P) -> Value
where
    P: AsRef<Path>,
{
    let contents = fs::read_to_string(atomic.as_ref()).expect("Unable to read atomic file");
    toml::from_str(&contents).expect("Unable to read atomic file")
}

#[allow(dead_code)]
fn parse_toml_value(parsed_toml: Value) {
    // Extract items from the TOML value
    match parsed_toml {
        Value::Table(table) => {
            // Iterate over key-value pairs in the table
            for (key, value) in &table {
                println!("{} : {}", key, value);
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

fn table_lookup<'a>(value: &'a Value, table_name: &str, key: &str) -> Option<&'a Value> {
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
