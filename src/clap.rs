use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use clap::Command;
use toml::Value;

fn cli() -> Command {
    Command::new("atomic")
        .about("auto local commit while testing without having to think about it")
        .allow_external_subcommands(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("init").about("initialize atomic template in project repository"))
        .subcommand(Command::new("watch").about("configure issue details for atomic commits"))
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

    // read in atomic file and parse it out

    // Deserialize the TOML data into a generic data structure
    let parsed_toml = get_toml_content(atomic);
    // parse_toml_value(parsed_toml.clone());

    // let res = table_lookup(&parsed_toml, "custom", "chain");

    let (table_name, value) =
        find_key_in_tables(parsed_toml.clone(), "chain").unwrap_or((String::new(), None));

    let sub_values = value.unwrap().as_array();

    // if let Some(sub_values) = value.as_array() {
    //     sub_values.into_iter().for_each(|v| {
    //         if let Some((_inner_table_name, Some(inner_value))) =
    //             find_key_in_tables(parsed_toml.clone(), v.as_str().unwrap())
    //         {
    //             println!("inner_inner value: {}", inner_value);
    //         }
    //     });
    // }
}

fn find_key_in_tables(parsed_toml: Value, key: &str) -> Option<(String, Option<Value>)> {
    if let Some(table) = parsed_toml.as_table() {
        for (k, v) in table {
            // println!("Found key '{}'; value: '{}'", k, v);
            if let Some(inner_table) = v.as_table() {
                if inner_table.contains_key(key) {
                    // println!(
                    // "The TOML value contains the key {key}\nv {:?}",
                    // inner_table.get(key)
                    // );
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
fn test_parse_toml_value(parsed_toml: Value) {
    // Extract items from the TOML value
    match parsed_toml {
        Value::Table(table) => {
            // Iterate over key-value pairs in the table
            for (key, value) in &table {
                // if key.eq(passed_cmd) {
                println!("found {}, v: {}", key, value);
                // }
            }
        }
        Value::Array(array) => {
            // Iterate over items in the array
            for item in array {
                println!("Array item: {:?}", item);
            }
        }
        Value::String(string_val) => {
            println!("String value: {}", string_val);
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
