use chrono::Local;
use toml::Value;
use crate::toml::get_toml_content;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
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
    let resolved = crate::plugin::resolve_script_path(&plugin.script, plugin.preferred.as_deref())?;

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
            format!("Plugin '{}' failed with exit code {:?}", name, status.code()),
        ))
    }
}


fn load_atomic_toml(path: &str) -> Result<Value> {
    get_toml_content(path).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "atomic.toml not found"))
}

fn parse_plugin_entry(name: &str, toml: &Value) -> Result<PluginConfig> {
    let plugin_section = toml
        .get("plugin")
        .and_then(|v| v.get(name))
        .and_then(|v| v.as_table())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("Plugin '{}' not found", name)))?;

    let script = plugin_section["script"]
        .as_str()
        .expect("plugin.script must be a string")
        .to_string();

    let preferred = plugin_section.get("preferred").and_then(|v| v.as_str()).map(String::from);

    let args = plugin_section
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let silent = plugin_section.get("silent").and_then(|v| v.as_bool()).unwrap_or(false);

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

fn run_plugin_silent(name: &str, stdout: impl io::Read + Send + 'static, stderr: impl io::Read + Send + 'static) -> Result<ExitStatus> {
    fs::create_dir_all("atomic-logs")?;
    let log_path = format!("atomic-logs/{}.log", name);
    let mut log_file = OpenOptions::new().create(true).append(true).open(&log_path)?;
    
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

fn run_plugin_stream(stdout: impl io::Read + Send + 'static, stderr: impl io::Read + Send + 'static) -> Result<ExitStatus> {
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
