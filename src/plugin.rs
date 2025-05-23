use crate::toml::get_toml_content;
use chrono::Local;
use lazy_static::lazy_static;
use serde::Deserialize;
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

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptEngine {
    pub ext: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub os: Option<String>, // "windows", "unix", or None for all
}

pub fn built_in_engines() -> Vec<ScriptEngine> {
    vec![
        ScriptEngine {
            ext: "bat".to_string(),
            program: "cmd".to_string(),
            args: vec!["/C".to_string()],
            description: "Windows Batch script".to_string(),
            os: Some("windows".to_string()),
        },
        ScriptEngine {
            ext: "cmd".to_string(),
            program: "cmd".to_string(),
            args: vec!["/C".to_string()],
            description: "Windows Command script".to_string(),
            os: Some("windows".to_string()),
        },
        ScriptEngine {
            ext: "ps1".to_string(),
            program: "powershell".to_string(),
            args: vec![
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
            ],
            description: "PowerShell script".to_string(),
            os: Some("windows".to_string()),
        },
        ScriptEngine {
            ext: "sh".to_string(),
            program: "sh".to_string(),
            args: vec![],
            description: "Unix Shell script".to_string(),
            os: Some("unix".to_string()),
        },
        ScriptEngine {
            ext: "py".to_string(),
            program: "python".to_string(),
            args: vec![],
            description: "Python script".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "go".to_string(),
            program: "go".to_string(),
            args: vec!["run".to_string()],
            description: "Go source file".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "js".to_string(),
            program: "node".to_string(),
            args: vec![],
            description: "Node.js JavaScript".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "mjs".to_string(),
            program: "node".to_string(),
            args: vec![],
            description: "Node.js ES module".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "ts".to_string(),
            program: "deno".to_string(),
            args: vec!["run".to_string()],
            description: "Deno TypeScript".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "rb".to_string(),
            program: "ruby".to_string(),
            args: vec![],
            description: "Ruby script".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "lua".to_string(),
            program: "lua".to_string(),
            args: vec![],
            description: "Lua script".to_string(),
            os: None,
        },
        ScriptEngine {
            ext: "exe".to_string(),
            program: "".to_string(),
            args: vec![],
            description: "Windows Executable".to_string(),
            os: Some("windows".to_string()),
        },
    ]
}

lazy_static! {
    pub static ref SUPPORTED_ENGINES: Vec<ScriptEngine> = built_in_engines();
}

/// Runs a plugin defined in `[plugin.<name>]` in atomic.toml.
pub fn run_plugin(name: &str, path: &str) -> Result<()> {
    let toml = load_atomic_toml(path)?;
    let plugin = parse_plugin_entry(name, &toml)?;
    let resolved = crate::plugin::resolve_script_path(&plugin.script, plugin.preferred.as_ref())?;

    let mut command = build_command(&resolved, &plugin.args);
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let status = if plugin.silent {
        run_plugin_silent(name, stdout, stderr)?
    } else {
        run_plugin_stream(stdout, stderr)?
    };

    if status.success() {
        println!("✅ Plugin '{name}' executed successfully.");
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
                format!("Plugin '{name}' not found"),
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
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);

    Ok(PluginConfig {
        script,
        args,
        preferred,
        silent,
    })
}

fn build_command(
    resolved: &crate::plugin::ScriptCommand,
    args: &[String],
) -> std::process::Command {
    let mut cmd = Command::new(&resolved.program);
    cmd.args(&resolved.args)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn run_plugin_silent(
    name: &str,
    stdout: impl io::Read + Send + 'static,
    stderr: impl io::Read + Send + 'static,
) -> Result<ExitStatus> {
    fs::create_dir_all("atomic-logs")?;
    let log_path = format!("atomic-logs/{name}.log");
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let out_thread = {
        let mut log_file = log_file.try_clone()?;
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let now = Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(log_file, "[{now}] [stdout] {line}").ok();
            }
        })
    };

    let err_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let now = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(log_file, "[{now}] [stderr] {line}").ok();
        }
    });

    let exit = Command::new("true").status()?;
    out_thread.join().ok();
    err_thread.join().ok();

    println!("Output logged to '{log_path}'");
    Ok(exit)
}

fn run_plugin_stream(
    stdout: impl io::Read + Send + 'static,
    stderr: impl io::Read + Send + 'static,
) -> Result<ExitStatus> {
    let out_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            println!("▶️ {line}");
        }
    });

    let err_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            eprintln!("❗ {line}");
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
            let full = format!("{base_path}.{ext}");
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
        eprintln!("⚠️ Multiple matching script types for '{base_path}': [{found}]. Using first.");
    }

    // Use first match or fail
    if let Some((ext, full)) = candidates.first() {
        return map_extension_to_command(full.clone(), ext);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No supported script found for '{base_path}'"),
    ))
}

pub fn map_extension_to_command(full_path: String, ext: &str) -> io::Result<ScriptCommand> {
    let engine = SUPPORTED_ENGINES.iter().find(|e| {
        e.ext == ext
            && (e.os.is_none()
                || (cfg!(windows) && e.os.as_deref() == Some("windows"))
                || (cfg!(unix) && e.os.as_deref() == Some("unix")))
    });

    match engine {
        Some(engine) => {
            let mut args = engine.args.clone();
            if engine.ext == "exe" {
                Ok(ScriptCommand {
                    program: full_path,
                    args: vec![],
                })
            } else {
                args.push(full_path);
                Ok(ScriptCommand {
                    program: engine.program.clone(),
                    args,
                })
            }
        }
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported script extension: .{ext} — define a plugin manually if needed."),
        )),
    }
}

fn supported_extensions(preferred: Option<&String>) -> Vec<String> {
    SUPPORTED_ENGINES
        .iter()
        .filter(|e| {
            e.os.is_none()
                || (cfg!(windows) && e.os.as_deref() == Some("windows"))
                || (cfg!(unix) && e.os.as_deref() == Some("unix"))
        })
        .filter(|e| preferred.map_or(true, |p| &e.ext == p))
        .map(|e| e.ext.clone())
        .collect()
}

/// Print all supported script extensions, for docs or help
pub fn print_supported_extensions() {
    println!("Supported script extensions:");
    for engine in SUPPORTED_ENGINES.iter() {
        if engine.os.is_none()
            || (cfg!(windows) && engine.os.as_deref() == Some("windows"))
            || (cfg!(unix) && engine.os.as_deref() == Some("unix"))
        {
            println!(
                ".{} — {} ({})",
                engine.ext, engine.description, engine.program
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn setup_mock_script(ext: &str) -> String {
        let filename = format!("../scripts/test.{}", ext);
        let content = match ext {
            "sh" => "#!/bin/sh\necho 'Hello from sh'",
            "py" => "print('Hello from Python')",
            "ps1" => "Write-Output \"Hello from PowerShell\"",
            "bat" => "@echo Hello from batch",
            _ => "echo Unsupported script",
        };
        fs::create_dir_all("../scripts").ok();
        let mut file = File::create(&filename).expect("Failed to create test script");
        writeln!(file, "{}", content).unwrap();
        filename
    }

    #[test]
    fn test_script_resolution_with_extension() {
        #[cfg(unix)]
        {
            // Ensure ../scripts exists and write a test.sh script
            let script_path = "./scripts/test.sh";
            std::fs::create_dir_all("./scripts").expect("Failed to create script dir");
            std::fs::write(script_path, "#!/bin/sh\necho 'Hello from shell script'")
                .expect("Failed to write test script");

            // Confirm that the .sh engine is included in supported engines
            let found = SUPPORTED_ENGINES.iter().any(|e| e.ext == "sh");
            assert!(found, "Built-in engines must include .sh");

            // Run the actual resolution logic
            let resolved =
                resolve_script_path(script_path, None).expect("Should resolve .sh script");

            // Validate resolution output
            assert_eq!(resolved.program, "sh", "Should use 'sh' as the shell");
            assert!(
                resolved.args.last().unwrap().ends_with("test.sh"),
                "Script path should be passed as final arg"
            );
        }

        #[cfg(windows)]
        {
            // On Windows, test with .bat instead
            let script_path = "./scripts/test.bat";
            std::fs::create_dir_all("./scripts").expect("Failed to create script dir");
            std::fs::write(script_path, "@echo Hello from batch script")
                .expect("Failed to write test script");

            let found = SUPPORTED_ENGINES.iter().any(|e| e.ext == "bat");
            assert!(found, "Built-in engines must include .bat");

            let resolved =
                resolve_script_path(script_path, None).expect("Should resolve .bat script");

            assert_eq!(
                resolved.program.to_lowercase(),
                "cmd",
                "Should use 'cmd' as the interpreter"
            );
            assert!(
                resolved.args.last().unwrap().ends_with("test.bat"),
                "Script path should be passed as final arg"
            );
        }
    }

    #[test]
    fn test_script_resolution_with_preferred() {
        let _ = setup_mock_script("py");
        let cmd = resolve_script_path("../scripts/test", Some(&"py".to_string()))
            .expect("Should resolve with preferred py extension");
        assert!(cmd.program.contains("python"));
        assert!(cmd.args.last().unwrap().ends_with("test.py"));
    }

    #[test]
    fn test_missing_script_fails() {
        let result = resolve_script_path("../scripts/does_not_exist", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_print_supported_extensions_runs() {
        print_supported_extensions(); // Should not panic
    }
}
