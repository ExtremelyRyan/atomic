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
            eprintln!("Failed to execute command: {cmd}\nError: {err}");
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
    let Some(branch_name) = head.shorthand() else {
        return Err(AtomicError::Static("Failed to get current branch name"));
    };

    // Parse branch name into parts
    let parts = parse_branch_name(branch_name)?; // Updated to handle Vec<String>

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
    index.add_all(std::iter::once(&"*"), git2::IndexAddOption::DEFAULT, None)?;

    let repo_reference = repo.head()?.resolve()?;
    let branch = repo_reference.name().expect("No HEAD exists");

    // Get the current user information from the Git configuration
    let config = repo.config()?;
    let user_name = config.get_string("user.name")?;
    let user_email = config.get_string("user.email")?;
    let user = Signature::now(&user_name, &user_email)?;

    // Generate commit message
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let message = commit_msg.map_or_else(
        || format!("[{timestamp}] atomic auto-commit"),
        |msg| format!("[{timestamp}] {msg}"),
    );

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

/// Entry point: Summarize all local changes into a single commit with a custom message, and force-push.
/// - Auto-commits any staged changes if there are no local commits.
/// - Squashes multiple commits or amends a single commit as needed.
/// - Always results in one commit on remote with your message.
pub fn summarize_and_push_commits(base_branch: &str, message: &str) -> Result<()> {
    let base_commit = find_merge_base(base_branch)?;
    let mut commit_count = count_commits_since(&base_commit)?;

    // Always stage all changes first.
    stage_all_changes()?;

    // After staging, commit staged changes if there are no commits yet.
    if commit_count == 0 {
        commit_staged_changes(message)?;
        // Check again after commit attempt:
        commit_count = count_commits_since(&base_commit)?;
        if commit_count == 0 {
            return Err(AtomicError::Static(
                "No commits, staged, or unstaged changes to squash/amend.",
            ));
        }
    }

    // Now, squash or amend as needed.
    if commit_count > 1 {
        squash_commits(&base_commit, message)?;
    } else {
        amend_last_commit(message)?;
    }

    force_push()?;
    Ok(())
}

/// Stages all changes (staged and unstaged) in the working directory.
fn stage_all_changes() -> Result<()> {
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to git add -A: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AtomicError::Static(
            "Failed to stage all changes with git add -A.",
        ))
    }
}

/// Finds the merge-base between HEAD and the given base branch.
fn find_merge_base(base_branch: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["merge-base", "HEAD", base_branch])
        .output()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git merge-base: {e}")))?;

    if !output.status.success() {
        return Err(AtomicError::Generic(format!(
            "Could not determine merge-base with branch '{}': {}",
            base_branch,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let commit = String::from_utf8(output.stdout)
        .map_err(|e| AtomicError::Generic(format!("Invalid UTF-8 in merge-base: {e}")))?;
    Ok(commit.trim().to_string())
}

/// Counts the number of commits between `base_commit` and HEAD.
fn count_commits_since(base_commit: &str) -> Result<usize> {
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{base_commit}..HEAD")])
        .output()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git rev-list: {e}")))?;

    let count_str = String::from_utf8(output.stdout)
        .map_err(|e| AtomicError::Generic(format!("Invalid UTF-8 in rev-list: {e}")))?;
    count_str
        .trim()
        .parse()
        .map_err(|e| AtomicError::Generic(format!("Failed to parse commit count: {e}")))
}

/// Returns true if there are staged changes to be committed.
fn has_staged_changes() -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to check staged changes: {e}")))?;
    Ok(!status.success())
}

/// Commits any staged changes using the provided message.
/// Returns Ok(()) even if there's nothing to commit (idempotent).
fn commit_staged_changes(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-am", message])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to commit staged changes: {e}")))?;
    // Accept both successful commit and "nothing to commit"
    if status.success() || status.code() == Some(1) {
        Ok(())
    } else {
        Err(AtomicError::Static(
            "Failed to create a commit from staged changes.",
        ))
    }
}

/// Squashes all commits since the merge-base into a single commit.
fn squash_commits(base_commit: &str, message: &str) -> Result<()> {
    // Soft-reset to the base commit
    let reset_status = Command::new("git")
        .args(["reset", "--soft", base_commit])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git reset: {e}")))?;
    if !reset_status.success() {
        return Err(AtomicError::Static("Failed to perform git reset --soft"));
    }

    // Commit everything with the provided message
    let commit_status = Command::new("git")
        .args(["commit", "-am", message])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git commit: {e}")))?;
    if commit_status.success() {
        Ok(())
    } else {
        Err(AtomicError::Static("Failed to create the squashed commit"))
    }
}

/// Amends the last commit with the provided message.
fn amend_last_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "--amend", "-m", message])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to amend the single commit: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AtomicError::Static("Failed to amend the single commit"))
    }
}

/// Force-pushes the current branch to remote using --force-with-lease.
fn force_push() -> Result<()> {
    let status = Command::new("git")
        .args(["push", "--force-with-lease"])
        .status()
        .map_err(|e| AtomicError::Generic(format!("Failed to run git push: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AtomicError::Static(
            "Failed to push branch after squashing/amending",
        ))
    }
}

pub fn parse_branch_name(branch_name: &str) -> Result<Vec<String>> {
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
        .map(std::string::ToString::to_string) // Convert to String
        .collect();

    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testparse_branch_name() {
        // Test parsing multiple parts
        assert_eq!(
            parse_branch_name("feature-144-adding_dark_mode"),
            Ok(vec![
                "feature".to_string(),
                "144".to_string(),
                "adding_dark_mode
                "
                .to_string()
            ])
        );

        // Test parsing two parts
        assert_eq!(
            parse_branch_name("feature-144"),
            Ok(vec!["feature".to_string(), "144".to_string()])
        );

        // Test parsing one part
        assert_eq!(
            parse_branch_name("feature"),
            Ok(vec!["feature".to_string()])
        );

        // Test empty branch name
        assert_eq!(
            parse_branch_name(""),
            Err(AtomicError::Static(
                "Branch name cannot be empty or contain only delimiters."
            ))
        );

        // Test invalid input (all delimiters)
        assert_eq!(
            parse_branch_name("---"),
            Err(AtomicError::Static(
                "Branch name cannot be empty or contain only delimiters."
            ))
        );

        // Test parsing many parts
        assert_eq!(
            parse_branch_name("feature-144-adding-dark-mode-extras"),
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
