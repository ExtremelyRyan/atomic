//! template.rs
//! 

use std::{
    io,
    path::{Path, PathBuf},
};

pub const RUST_TEMPLATE: &str = include_str!("../template/rust.toml");
pub const GENERIC_TEMPLATE: &str = include_str!("../template/example.toml");

pub fn user_template_path(name: &str) -> Option<PathBuf> {
    let base = dirs::config_dir()?; // ~/.config or %APPDATA%
    Some(
        base.join("atomic")
            .join("templates")
            .join(format!("{name}.toml")),
    )
}

pub fn _save_template(name: &str, source: &str) -> io::Result<()> {
    let Some(path) = user_template_path(name) else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Could not resolve config directory",
        ));
    };

    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::copy(source, &path)?;

    println!("✅ Saved template as '{}'", path.display());
    Ok(())
}

/// Returns the first valid example file found: rust.toml or example.toml
fn _find_template_file() -> io::Result<PathBuf> {
    let candidates = [
        Path::new("example/rust.toml"),
        Path::new("example/example.toml"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No template file found in ./example/",
    ))
}
