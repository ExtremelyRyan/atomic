use crate::toml::get_toml_content;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use toml::Value;
type Result<T> = std::result::Result<T, io::Error>;

/// Resolves the correct script file based on platform and extension priorities.
pub struct ScriptCommand {
    program: String,
    args: Vec<String>,
}

struct PluginConfig {
    script: String,
    args: Vec<String>,
    preferred: Option<String>,
    silent: bool,
}

/// Runs a plugin defined in `[plugin.<name>]` in atomic.toml.
pub fn run_plugin(name: &str, path: &str) -> Result<()> {
    let toml = load_atomic_toml(path)?;
    let plugin = parse_plugin_entry(name, &toml)?;
    let resolved = crate::plugin::resolve_script_path(&plugin.script, plugin.preferred.as_ref())?;

    let mut command = build_command(&resolved, &plugin.args)?;
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let status = if plugin.silent {
        run_plugin_silent(name, stdout, stderr)?
    } else {
        run_plugin_stream(stdout, stderr)?
    };

    if status.success() {
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
    }
}

fn load_atomic_toml(path: &str) -> Result<Value> {
    get_toml_content(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "atomic.toml not found"))
}

fn parse_plugin_entry(name: &str, toml: &Value) -> Result<PluginConfig> {
    let plugin_section = toml
        .get("plugin")
        .and_then(|v| v.get(name))
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Plugin '{}' not found", name),
            )
        })?;

    let script = plugin_section["script"]
        .as_str()
        .expect("plugin.script must be a string")
        .to_string();

    let preferred = plugin_section
        .get("preferred")
        .and_then(|v| v.as_str())
        .map(String::from);

    let args = plugin_section
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let silent = plugin_section
        .get("silent")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(PluginConfig {
        script,
        preferred,
        args,
        silent,
    })
}

fn build_command(resolved: &crate::plugin::ScriptCommand, args: &[String]) -> Result<Command> {
    let mut cmd = Command::new(&resolved.program);
    cmd.args(&resolved.args)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(cmd)
}

fn run_plugin_silent(
    name: &str,
    stdout: impl io::Read + Send + 'static,
    stderr: impl io::Read + Send + 'static,
) -> Result<ExitStatus> {
    fs::create_dir_all("atomic-logs")?;
    let log_path = format!("atomic-logs/{}.log", name);
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let out_thread = {
        let mut log_file = log_file.try_clone()?;
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                let now = Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(log_file, "[{}] [stdout] {}", now, line).ok();
            }
        })
    };

    let err_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            let now = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(log_file, "[{}] [stderr] {}", now, line).ok();
        }
    });

    let exit = Command::new("true").status()?;
    out_thread.join().ok();
    err_thread.join().ok();

    println!("Output logged to '{}'", log_path);
    Ok(exit)
}

fn run_plugin_stream(
    stdout: impl io::Read + Send + 'static,
    stderr: impl io::Read + Send + 'static,
) -> Result<ExitStatus> {
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

    let status = Command::new("true").status()?;
    out_thread.join().ok();
    err_thread.join().ok();
    Ok(status)
}

/// Resolves a script path from atomic.toml into an executable command,
/// using its extension, platform, and user-specified preference.
///
/// - If `base_path` has an extension, it’s resolved directly.
/// - If not, we try known extensions (platform-aware) and match the first valid file.
///
/// # Arguments
/// * `base_path` - Script path from TOML, with or without extension.
/// * `preferred` - Optional preferred extension (e.g. "ps1", "py")
pub fn resolve_script_path(
    base_path: &str,
    preferred: Option<&String>,
) -> io::Result<ScriptCommand> {
    let path = Path::new(base_path);

    // Shortcut: if file already has an extension, resolve directly
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        return map_extension_to_command(base_path.to_string(), ext);
    }

    // Otherwise, try supported extensions dynamically
    let supported = supported_extensions(preferred);
    let candidates: Vec<_> = supported
        .iter()
        .filter_map(|ext| {
            let full = format!("{}.{}", base_path, ext);
            if fs::metadata(&full).is_ok() {
                Some((ext.as_str(), full))
            } else {
                None
            }
        })
        .collect();

    // Warn if multiple matching files are found
    if candidates.len() > 1 {
        let found = candidates
            .iter()
            .map(|(ext, _)| *ext)
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "⚠️ Multiple matching script types for '{}': [{}]. Using first.",
            base_path, found
        );
    }

    // Use first match or fail
    if let Some((ext, full)) = candidates.first() {
        return map_extension_to_command(full.clone(), ext);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No supported script found for '{}'", base_path),
    ))
}

/// Maps a file extension to a platform-aware command runner.
///
/// You can add more entries here if needed, but this covers:
/// - Bash/shell
/// - PowerShell and batch scripts
/// - Python, Node, Deno, Ruby
/// - Raw executables
pub fn map_extension_to_command(full_path: String, ext: &str) -> io::Result<ScriptCommand> {
    let command = match ext {
        // Windows batch / legacy
        "bat" | "cmd" => ScriptCommand {
            program: "cmd".into(),
            args: vec!["/C".into(), full_path],
        },

        // PowerShell script
        "ps1" => ScriptCommand {
            program: "powershell".into(),
            args: vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                full_path,
            ],
        },

        // Shell / Bash
        "sh" => ScriptCommand {
            program: "sh".into(),
            args: vec![full_path],
        },

        // Python
        "py" => ScriptCommand {
            program: "python".into(),
            args: vec![full_path],
        },

        // Golang
        "go" => ScriptCommand {
            program: "go".into(),
            args: vec!["run".into(), full_path],
        },

        // Node.js
        "js" | "mjs" => ScriptCommand {
            program: "node".into(),
            args: vec![full_path],
        },

        // Deno
        "ts" => ScriptCommand {
            program: "deno".into(),
            args: vec!["run".into(), full_path],
        },

        // Ruby
        "rb" => ScriptCommand {
            program: "ruby".into(),
            args: vec![full_path],
        },

        // Lua
        "lua" => ScriptCommand {
            program: "lua".into(),
            args: vec![full_path],
        },

        // Executables — run directly
        "exe" => ScriptCommand {
            program: full_path,
            args: vec![],
        },

        // Fallthrough: not supported
        unsupported => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Unsupported script extension: .{} — define a plugin manually if needed.",
                    unsupported
                ),
            ));
        }
    };

    Ok(command)
}

/// Returns a prioritized list of script extensions based on the current OS and user preference.
/// This list should align with what `map_extension_to_command()` supports.
pub fn supported_extensions(preferred: Option<&String>) -> Vec<String> {
    // Windows platform extensions
    let windows_defaults = ["bat", "cmd", "ps1", "exe", "py", "js", "ts", "rb", "lua", "go"];

    // Unix/Linux platform extensions
    let unix_defaults = ["sh", "py", "js", "ts", "rb", "lua", "go"];

    let mut ordered = match cfg!(windows) {
        true => Vec::with_capacity(windows_defaults.len()),
        false => Vec::with_capacity(unix_defaults.len()),
    };

    // If user has a preferred extension, push it to the front
    if let Some(p) = preferred {
        ordered.push(p.to_string());
    }

    // Append remaining extensions, skipping duplicates
    ordered.extend(
        ordered
            .clone()
            .iter()
            .filter(|ext| Some(*ext) != preferred)
            .map(|ext| ext.to_string()),
    );

    ordered
}
