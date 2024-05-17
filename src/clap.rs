use std::{fs::OpenOptions, io::Read};

use clap::{arg, value_parser, Command};
use toml::Value;

fn cli() -> Command {
    Command::new("atomic")
        .about("auto local commit while testing without having to think about it")
        .allow_external_subcommands(true)
        .subcommand(Command::new("init").about("initialize atomic template in project repository"))
        .subcommand(
            Command::new("watch")
                .about("configure issue details for atomic commits")
                // .arg(arg!([issue] "track issue number (u64)").value_parser(value_parser!(u64)))
                // .arg_required_else_help(false)
                // .arg(
                //     arg!([desc] "brief description of problem").value_parser(value_parser!(String)),
                // ),
        )
    // .subcommand(
    //     Command::new("diff")
    //         .about("Compare two commits")
    //         .arg(arg!(base: [COMMIT]))
    //         .arg(arg!(head: [COMMIT]))
    //         .arg(arg!(path: [PATH]).last(true))
    //         .arg(
    //             arg!(--color <WHEN>)
    //                 .value_parser(["always", "auto", "never"])
    //                 .num_args(0..=1)
    //                 .require_equals(true)
    //                 .default_value("auto")
    //                 .default_missing_value("always"),
    //         ),
    // )
    // .subcommand(
    //     Command::new("push")
    //         .about("pushes things")
    //         .arg(arg!(<REMOTE> "The remote to target"))
    //         .arg_required_else_help(true),
    // )
    // .subcommand(
    //     Command::new("add")
    //         .about("adds things")
    //         .arg_required_else_help(true)
    //         .arg(arg!(<PATH> ... "Stuff to add").value_parser(clap::value_parser!(PathBuf))),
    // )
    // .subcommand(
    //     Command::new("stash")
    //         .args_conflicts_with_subcommands(true)
    //         .flatten_help(true)
    //         .args(push_args())
    //         .subcommand(Command::new("push").args(push_args()))
    //         .subcommand(Command::new("pop").arg(arg!([STASH])))
    //         .subcommand(Command::new("apply").arg(arg!([STASH]))),
    // )
}

// fn push_args() -> Vec<clap::Arg> {
//     vec![arg!(-m --message <MESSAGE>)]
// }

pub fn start_cli() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", sub_matches)) => {
            start_init();
        }
        // Some(("clone", sub_matches)) => {
        //     println!(
        //         "Cloning {}",
        //         sub_matches.get_one::<String>("REMOTE").expect("required")
        //     );
        // }
        // Some(("diff", sub_matches)) => {
        //     let color = sub_matches
        //         .get_one::<String>("color")
        //         .map(|s| s.as_str())
        //         .expect("defaulted in clap");

        //     let mut base = sub_matches.get_one::<String>("base").map(|s| s.as_str());
        //     let mut head = sub_matches.get_one::<String>("head").map(|s| s.as_str());
        //     let mut path = sub_matches.get_one::<String>("path").map(|s| s.as_str());
        //     if path.is_none() {
        //         path = head;
        //         head = None;
        //         if path.is_none() {
        //             path = base;
        //             base = None;
        //         }
        //     }
        //     let base = base.unwrap_or("stage");
        //     let head = head.unwrap_or("worktree");
        //     let path = path.unwrap_or("");
        //     println!("Diffing {base}..{head} {path} (color={color})");
        // }
        // Some(("push", sub_matches)) => {
        //     println!(
        //         "Pushing to {}",
        //         sub_matches.get_one::<String>("REMOTE").expect("required")
        //     );
        // }
        // Some(("add", sub_matches)) => {
        //     let paths = sub_matches
        //         .get_many::<PathBuf>("PATH")
        //         .into_iter()
        //         .flatten()
        //         .collect::<Vec<_>>();
        //     println!("Adding {paths:?}");
        // }
        // Some(("stash", sub_matches)) => {
        //     let stash_command = sub_matches.subcommand().unwrap_or(("push", sub_matches));
        //     match stash_command {
        //         ("apply", sub_matches) => {
        //             let stash = sub_matches.get_one::<String>("STASH");
        //             println!("Applying {stash:?}");
        // Some(_) => todo!(),
        // None => todo!(),
        //         }
        //         ("pop", sub_matches) => {
        //             let stash = sub_matches.get_one::<String>("STASH");
        //             println!("Popping {stash:?}");
        //         }
        //         ("push", sub_matches) => {
        //             let message = sub_matches.get_one::<String>("message");
        //             println!("Pushing {message:?}");
        //         }
        //         (name, _) => {
        //             unreachable!("Unsupported subcommand `{name}`")
        //         }
        //     }
        // }
        // Some((ext, sub_matches)) => {
        //     let args = sub_matches
        //         .get_many::<OsString>("")
        //         .into_iter()
        //         .flatten()
        //         .collect::<Vec<_>>();
        //     println!("Calling out to {ext:?} with {args:?}");
        _ => (), // If all subcommands are defined above, anything else is unreachable!()
    }
}

fn start_init() {
    // check for atomic file, and if does not exist, create.
    let mut atomic = if let Ok(atomic_file) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("atomic.toml")
    {
        debug_assert!(&atomic_file.metadata().unwrap().is_file());
        atomic_file
    } else {
        eprintln!("error creating atomic file");
        return;
    };

    let mut contents = String::new();

    atomic.read_to_string(&mut contents).unwrap();

    // Deserialize the TOML data into a generic data structure
    let parsed_toml = toml::from_str::<Value>(&contents).expect("Unable to parse TOML");

    let passed_cmd = "run";

    // Extract items from the TOML value
    match parsed_toml {
        toml::Value::Table(table) => {
            // Iterate over key-value pairs in the table
            for (key, value) in &table {
                if key.eq(passed_cmd) {
                    println!("found {}, v: {}", key, value);
                }
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
