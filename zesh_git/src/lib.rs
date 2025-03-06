use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("failed to execute command: {0}")]
    IoError(#[from] std::io::Error),

    #[error("git command error: {0}")]
    CommandError(String),
}

/// A trait representing Git operations.
pub trait Git {
    /// Runs `git rev-parse --show-toplevel` in the given directory.
    /// Returns a tuple where the first element is `true` if the command succeeded,
    /// and the second element is either the top-level directory path or the error output.
    fn show_top_level(&self, name: &str) -> Result<(bool, String), GitError>;

    /// Runs `git rev-parse --git-common-dir` in the given directory.
    /// Returns a tuple where the first element is `true` if the command succeeded,
    /// and the second element is either the common directory path or the error output.
    fn git_common_dir(&self, name: &str) -> Result<(bool, String), GitError>;

    /// Runs `git clone <url> <dir>` in the given command directory.
    /// Returns the output string on success.
    fn clone(&self, url: &str, cmd_dir: &str, dir: &str) -> Result<String, GitError>;
}

/// A real implementation of the Git trait that calls the actual git commands.
pub struct RealGit;

impl Git for RealGit {
    fn show_top_level(&self, name: &str) -> Result<(bool, String), GitError> {
        let output = Command::new("git")
            .args(["-C", name, "rev-parse", "--show-toplevel"])
            .output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok((true, stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Ok((false, stderr))
        }
    }

    fn git_common_dir(&self, name: &str) -> Result<(bool, String), GitError> {
        let output = Command::new("git")
            .args(["-C", name, "rev-parse", "--git-common-dir"])
            .output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok((true, stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Ok((false, stderr))
        }
    }

    fn clone(&self, url: &str, cmd_dir: &str, dir: &str) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(["clone", url, dir])
            .current_dir(cmd_dir)
            .output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::CommandError(stderr))
        }
    }
}

/// A mocked implementation of the Git trait for testing purposes.
pub struct MockGit;

impl Git for MockGit {
    fn show_top_level(&self, _name: &str) -> Result<(bool, String), GitError> {
        // Always return a mocked top-level directory.
        Ok((true, String::from("/mock/repo/top-level")))
    }

    fn git_common_dir(&self, _name: &str) -> Result<(bool, String), GitError> {
        // Always return a mocked common directory.
        Ok((true, String::from("/mock/repo/common-dir")))
    }

    fn clone(&self, _url: &str, _cmd_dir: &str, _dir: &str) -> Result<String, GitError> {
        // Always return a success message.
        Ok(String::from("Mock clone successful"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_git() {
        let git = MockGit;

        let (success, top_level) = git.show_top_level("any_dir").unwrap();
        assert!(success);
        assert_eq!(top_level, "/mock/repo/top-level");

        let (success, common_dir) = git.git_common_dir("any_dir").unwrap();
        assert!(success);
        assert_eq!(common_dir, "/mock/repo/common-dir");

        let clone_output = git
            .clone("https://example.com/repo.git", ".", "repo")
            .unwrap();
        assert_eq!(clone_output, "Mock clone successful");
    }
}
