use crate::{AtomicError, Result};
use git2::{Repository, Signature};
use std::env;
use std::process::{Command, Stdio};

const _SEPERATORS: [char; 4] = ['-', ' ', ':', '_'];

pub fn send_command(cmd: &str) {
    #[cfg(debug_assertions)]
    dbg!(cmd);

    // Handle empty or invalid commands
    if cmd.trim().is_empty() {
        println!("No command provided or unknown command.");
        return;
    }

    // Normalize quotes for Windows compatibility
    #[cfg(target_os = "windows")]
    let cmd = cmd.replace('\'', "\""); // Replace single quotes with double quotes

    // println!("Running command: {}", cmd);

    // Build the command based on the OS
    let mut process = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &cmd]) // Use /C for Windows
            .stdout(Stdio::inherit()) // Inherit stdout
            .stderr(Stdio::inherit()); // Inherit stderr
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &cmd]) // Use -c for Unix-like systems
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        c
    };

    // Execute the command and handle results
    match process.output() {
        Ok(output) => {
            // Check for success or failure status
            if !output.status.success() {
                eprintln!(
                    "Command failed with status code: {}",
                    output.status.code().unwrap_or(-1)
                );
            }
        }
        Err(err) => {
            // Handle execution errors
            eprintln!("Failed to execute command: {}\nError: {}", cmd, err);
        }
    }
}

pub fn _get_git_info() -> Result<(String, String, u64)> {
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Open the repository
    let repo = Repository::open(&current_dir)?;

    // Get the current branch name
    let head = repo.head().expect("Failed to get HEAD reference");
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => return Err(AtomicError::Static("Failed to get current branch name")),
    };

    // Parse branch name into parts
    let parts = _parse_branch_name(branch_name)?; // Updated to handle Vec<String>

    // Extract parts safely
    let feature = parts.first().cloned().unwrap_or_default(); // First part as feature
    let issue = parts.get(1).cloned().unwrap_or_default(); // Second part as issue number
    let desc = parts
        .get(2..)
        .map(|rest| rest.join("-"))
        .unwrap_or_default(); // Remaining parts as description

    // Parse issue number safely
    let issue_num = issue.parse::<u64>().unwrap_or(0);

    // Return feature, description, and issue number
    Ok((feature, desc, issue_num))
}

pub fn commit_local_changes(commit_msg: Option<&str>) -> Result<()> {
    let repo = Repository::open(".")?;
    let mut index = repo.index()?;

    // Add all changes to the index (staging area)
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;

    let repo_reference = repo.head()?.resolve()?;
    let branch = repo_reference.name().expect("No HEAD exists");

    // Get the current user information from the Git configuration
    let config = repo.config()?;
    let user_name = config.get_string("user.name")?;
    let user_email = config.get_string("user.email")?;
    let user = Signature::now(&user_name, &user_email)?;

    // Generate commit message
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let message = match commit_msg {
        Some(msg) => format!("[{}] {}", timestamp, msg),
        None => format!("[{}] atomic auto-commit", timestamp),
    };

    // Write the tree and create commit
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = repo.find_commit(repo.head()?.peel_to_commit()?.id())?;

    repo.commit(
        Some(branch),
        &user,
        &user,
        &message,
        &tree,
        &[&parent_commit],
    )?;

    Ok(())
}

/// Squash all local commits onto the given base branch, using the provided commit message.
/// Only safe if you haven't pushed those commits!
pub fn squash_local_commits(base_branch: &str, message: &str) -> Result<()> {
    // 1. Find the merge base with the target branch
    let merge_base = Command::new("git")
        .args(["merge-base", "HEAD", base_branch])
        .output()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git merge-base: {e}")))?;

    if !merge_base.status.success() {
        return Err(AtomicError::Generic(format!(
            "Could not determine merge-base with branch '{}': {}",
            base_branch,
            String::from_utf8_lossy(&merge_base.stderr)
        )));
    }
    let base_commit = String::from_utf8(merge_base.stdout)
        .map_err(|e| AtomicError::Generic(format!("Invalid UTF-8 in merge-base: {e}")))?;
    let base_commit = base_commit.trim();

    // 2. Soft reset to merge base (preserve working tree, stage all changes)
    let reset_status = Command::new("git")
        .args(["reset", "--soft", base_commit])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git reset: {e}")))?;
    if !reset_status.success() {
        return Err(AtomicError::Static("Failed to perform git reset --soft"));
    }

    // 3. Commit everything with the provided message
    let commit_status = Command::new("git")
        .args(["commit", "-am", message])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git commit: {e}")))?;
    if !commit_status.success() {
        return Err(AtomicError::Static("Failed to create the squashed commit"));
    }

    Ok(())
}


pub fn _parse_branch_name(branch_name: &str) -> Result<Vec<String>> {
    // Check if the branch name is empty or contains only delimiters
    if branch_name.trim().is_empty() || branch_name.chars().all(|c| c == '-')
    // Check for only delimiters
    {
        return Err(AtomicError::Static(
            "Branch name cannot be empty or contain only delimiters.",
        ));
    }

    // Parse parts by splitting on '-'
    let parts: Vec<String> = branch_name
        .split('-') // Split by '-'
        .filter(|s| !s.is_empty()) // Filter out empty parts
        .map(|s| s.to_string()) // Convert to String
        .collect();

    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branch_name() {
        // Test parsing multiple parts
        assert_eq!(
            _parse_branch_name("feature-144-adding_dark_mode"),
            Ok(vec![
                "feature".to_string(),
                "144".to_string(),
                "adding_dark_mode
                ".to_string()
            ])
        );

        // Test parsing two parts
        assert_eq!(
            _parse_branch_name("feature-144"),
            Ok(vec!["feature".to_string(), "144".to_string()])
        );

        // Test parsing one part
        assert_eq!(
            _parse_branch_name("feature"),
            Ok(vec!["feature".to_string()])
        );

        // Test empty branch name
        assert_eq!(
            _parse_branch_name(""),
            Err(AtomicError::Static(
                "Branch name cannot be empty or contain only delimiters."
            ))
        );

        // Test invalid input (all delimiters)
        assert_eq!(
            _parse_branch_name("---"),
            Err(AtomicError::Static(
                "Branch name cannot be empty or contain only delimiters."
            ))
        );

        // Test parsing many parts
        assert_eq!(
            _parse_branch_name("feature-144-adding-dark-mode-extras"),
            Ok(vec![
                "feature".to_string(),
                "144".to_string(),
                "adding".to_string(),
                "dark".to_string(),
                "mode".to_string(),
                "extras".to_string()
            ])
        );
    }
}

