use crate::{AtomicError::*, Result};
use git2::Repository;
use std::env;
use std::process::{Command, Stdio};

pub fn send_command<'a>(command: &'a str) {
    // dbg!(&command);
    if command.is_empty() {
        println!("unknown or no command was found");
        return;
    }

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", command])
            .env("FORCE_COLOR", "1")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        c
    };
    let status = cmd.status().expect("failed to execute process");

    if !status.success() {
        eprintln!("Command failed with exit status: {}", status);
    }
}
const _SEPERATORS: [char; 4] = ['-', ' ', ':', '_'];

pub fn get_git_info() -> Result<(String, String, u64)> {
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Open the repository
    let repo = match Repository::open(&current_dir) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open repository: {}", e),
    };

    // Get the current branch name
    let head = repo.head().expect("Failed to get HEAD reference");
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => panic!("Failed to get current branch name"),
    };

    let (feature, issue, description) = match parse_branch_name(branch_name)? {
        (Some(feature), None, None) => (feature, None, None),
        (Some(feature), Some(issue), None) => (feature, Some(issue), None),
        (Some(feature), Some(issue), Some(desc)) => (feature, Some(issue), Some(desc)),
        (Some(feature), ..) => (feature, None, None),
        _ => (String::from(""), None, None),
    };

    let desc = description.unwrap_or_default();

    let issue_num = match issue.unwrap_or_default().parse::<u64>() {
        Ok(num) => num,
        Err(_) => 0,
    };
    // Print the current branch and issue number
    // dbg!(&feature, &issue_num, &desc);

    Ok((feature, desc, issue_num))
}

/// Parses a Git branch name and extracts its components.
///
/// # Arguments
///
/// * `branch_name` - The name of the Git branch.
///
/// # Example
/// ```
/// use std::error::Error;
/// # use cargo_atomic::parse_branch_name;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let branch_name = "feature-144-adding_dark_mode";
///     let (feature, issue, description) = parse_branch_name(branch_name)?;
///     assert_eq!(Some("feature".to_string()), feature);
///     assert_eq!(Some("144".to_string()), issue);
///     assert_eq!(Some("adding_dark_mode".to_string()), description);
///     Ok(())
/// }
/// ```
///
/// # Returns
///
/// Returns a tuple containing the feature name, issue number, and description of the branch,
/// wrapped in a `Result`. If the branch name contains too many parts, an error is returned.
pub fn parse_branch_name(
    branch_name: &str,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let parts: Vec<&str> = branch_name.split('-').collect();
    match parts.len() {
        1 => Ok((Some(parts[0].to_string()), None, None)),
        2 => Ok((Some(parts[0].to_string()), Some(parts[1].to_string()), None)),
        3 => Ok((
            Some(parts[0].to_string()),
            Some(parts[1].to_string()),
            Some(parts[2].to_string()),
        )),
        _ => Err(Static("too many parts in branch name, maximum 3.")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branch_name() {
        assert_eq!(
            parse_branch_name("feature-144-adding_dark_mode"),
            Ok((
                Some("feature".to_string()),
                Some("144".to_string()),
                Some("adding_dark_mode".to_string())
            ))
        );
        assert_eq!(
            parse_branch_name("feature-144"),
            Ok((Some("feature".to_string()), Some("144".to_string()), None))
        );
        assert_eq!(
            parse_branch_name("feature"),
            Ok((Some("feature".to_string()), None, None))
        );
        assert!(parse_branch_name("feature-144-adding_dark_mode-extras").is_err());
    }
}
