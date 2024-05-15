use clap::start_cli;
use notify::{Config, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

mod clap;

fn main() {
    // send_command();
    start_cli();
}

fn send_command() {
    use std::process::Command;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "whoami"])
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("echo hello")
            .output()
            .expect("failed to execute process")
    };

    let hello = output.stdout;

    let out = String::from_utf8(hello).unwrap();
    println!("{}", out);
}

fn _watch_folder() {
    // Define the path to watch
    let path = Path::new("target");

    // Create a channel to receive events
    let (tx, rx) = channel();

    // Create a watcher object, the `tx` is used to send events to the channel

    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        // custom config for PollWatcher kind
        // you
        let config = Config::default().with_poll_interval(Duration::from_secs(1));
        Box::new(PollWatcher::new(tx, config).unwrap())
    } else {
        // use default config for everything else
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };
    // Watch the path recursively for any modifications
    watcher.watch(path, RecursiveMode::Recursive).unwrap();

    println!("Watching {} for changes...", path.display());

    // Start an infinite loop to receive events
    loop {
        match rx.recv() {
            Ok(event) => {
                // Handle the event here
                // For example, if you only want to know if something changed, you can print a message
                println!("Something changed in the watched folder!");
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
