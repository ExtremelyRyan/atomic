use notify::{Config, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

fn main() {
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
