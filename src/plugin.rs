use crate::toml::get_toml_content;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

/// Runs a plugin defined in `[plugin.<name>]` in atomic.toml.
///
/// If `silent = true` is set for the plugin:
/// - Output will not be shown in the terminal.
/// - Output will be logged to `atomic-logs/<plugin-name>.log`.
///
/// If `silent` is false or missing:
/// - Output will be streamed live to the terminal.
pub fn run_plugin(name: &str, path: &str) -> io::Result<()> {
    // Load and parse atomic.toml
    let Some(toml) = get_toml_content(path) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "atomic.toml not found",
        ));
    };

    // Locate the plugin definition under [plugin.<name>]
    let plugin_section = toml
        .get("plugin")
        .and_then(|v| v.get(name))
        .and_then(|entry| entry.as_table())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Plugin '{}' not found", name),
            )
        })?;

    // Required: path to script, without or with extension
    let base_script = plugin_section
        .get("script")
        .and_then(|v| v.as_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing 'script' in plugin"))?;

    // Optional: preferred extension (e.g. ps1, py)
    let preferred = plugin_section.get("preferred").and_then(|v| v.as_str());

    // Optional: arguments to pass to the plugin script
    let args: Vec<String> = plugin_section
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Optional: silence all output and log it to file instead of terminal
    let silent = plugin_section
        .get("silent")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve the full command + args to run (based on script type/platform)
    let resolved = resolve_script_path(base_script, preferred)?;

    // Start building the command
    let mut command = Command::new(&resolved.program);
    command.args(&resolved.args).args(&args);

    // Always pipe output to capture, whether we stream or log
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Start the plugin process
    let mut child = command.spawn()?;

    // Take control of stdout/stderr from the child
    let stdout = child.stdout.take().expect("stdout missing");
    let stderr = child.stderr.take().expect("stderr missing");

    if silent {
        // Create the log directory if it doesn't exist
        fs::create_dir_all("atomic-logs")?;

        // Append output to `atomic-logs/<plugin-name>.log`
        let log_path = format!("atomic-logs/{}.log", name);
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        // Clone log handle for writing stdout in a separate thread
        let out_thread = thread::spawn({
            let mut log_file = log_file.try_clone()?;
            move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                    writeln!(log_file, "[{}] [stdout] {}", now, line).ok();
                }
            }
        });

        // Handle stderr in a separate thread
        let err_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(log_file, "[{}] [stderr] {}", now, line).ok();
            }
        });

        // Wait for process and log threads to finish
        let status = child.wait()?;
        out_thread.join().ok();
        err_thread.join().ok();

        // Print log location
        println!("📁 Output logged to '{}'", log_path);

        // Return success/failure based on exit code
        return if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Plugin '{}' failed (logged). Exit code: {:?}",
                    name,
                    status.code()
                ),
            ))
        };
    } else {
        // Stream output directly to terminal in real-time
        let out_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                println!("▶️ {}", line);
            }
        });

        let err_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                eprintln!("❗ {}", line);
            }
        });

        // Wait for process and output threads to finish
        let status = child.wait()?;
        out_thread.join().ok();
        err_thread.join().ok();

        return if status.success() {
            println!("✅ Plugin '{}' executed successfully.", name);
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Plugin '{}' failed with exit code {:?}",
                    name,
                    status.code()
                ),
            ))
        };
    }
}

/// Resolves the correct script file based on platform and extension priorities.
pub struct ScriptCommand {
    program: String,
    args: Vec<String>,
}

/// Resolves a script path by checking extension, platform, and user preference.
/// Supports explicit file paths or extensionless base paths + dynamic resolution.
///
/// # Arguments
/// * `base_path` - The path from `atomic.toml`, with or without extension
/// * `preferred` - Optional user-specified preferred extension (e.g., "ps1", "py")
pub fn resolve_script_path(base_path: &str, preferred: Option<&str>) -> io::Result<ScriptCommand> {
    let path = Path::new(base_path);

    // If user gave an explicit file path with extension, use it directly
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let full = base_path.to_string();
        return match ext {
            "bat" | "cmd" => Ok(ScriptCommand {
                program: "cmd".into(),
                args: vec!["/C".into(), full],
            }),
            "ps1" => Ok(ScriptCommand {
                program: "powershell".into(),
                args: vec![
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-File".into(),
                    full,
                ],
            }),
            "sh" => Ok(ScriptCommand {
                program: "sh".into(),
                args: vec![full],
            }),
            "py" => Ok(ScriptCommand {
                program: "python".into(),
                args: vec![full],
            }),
            "exe" => Ok(ScriptCommand {
                program: full,
                args: vec![],
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported script extension: .{}", ext),
            )),
        };
    }

    // No extension given — resolve dynamically
    let supported = supported_extensions(preferred);
    let existing: Vec<_> = supported
        .iter()
        .filter_map(|ext| {
            let full = format!("{}.{}", base_path, ext);
            if fs::metadata(&full).is_ok() {
                Some((ext.to_string(), full))
            } else {
                None
            }
        })
        .collect();

    // Warn if multiple files found
    if existing.len() > 1 {
        let found = existing
            .iter()
            .map(|(e, _)| e.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "⚠️ Multiple script types found for '{}': [{}]. Using preferred or first match.",
            base_path, found
        );
    }

    // Build the appropriate command based on matched extension
    if let Some((ext, full)) = existing.first() {
        let command = match ext.as_str() {
            "bat" | "cmd" => ScriptCommand {
                program: "cmd".into(),
                args: vec!["/C".into(), full.clone()],
            },
            "ps1" => ScriptCommand {
                program: "powershell".into(),
                args: vec![
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-File".into(),
                    full.clone(),
                ],
            },
            "sh" => ScriptCommand {
                program: "sh".into(),
                args: vec![full.clone()],
            },
            "py" => ScriptCommand {
                program: "python".into(),
                args: vec![full.clone()],
            },
            "exe" => ScriptCommand {
                program: full.clone(),
                args: vec![],
            },
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported extension: .{}", ext),
                ));
            }
        };

        return Ok(command);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No supported script found for '{}'", base_path),
    ))
}

fn supported_extensions(preferred: Option<&str>) -> Vec<String> {
    let default = if cfg!(windows) {
        vec!["bat", "cmd", "ps1", "exe", "py"]
    } else {
        vec!["sh", "py"]
    };

    match preferred {
        Some(p) => {
            let mut ordered = vec![p.to_string()];
            ordered.extend(default.into_iter().filter(|e| *e != p).map(String::from));
            ordered
        }
        None => default.into_iter().map(String::from).collect(),
    }
}
