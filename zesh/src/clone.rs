use std::path::PathBuf;
use thiserror::Error;
use zesh_git::{Git, GitError};

use crate::fs::{FsError, FsOperations};
use zellij_rs::{ZellijError, ZellijOperations, options::ZellijOptions};
use zox_rs::{ZoxideError, ZoxideOperations};

#[derive(Debug, Error)]
pub enum CloneError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Zellij error: {0}")]
    Zellij(#[from] ZellijError),

    #[error("Zoxide error: {0}")]
    Zoxide(#[from] ZoxideError),

    #[error("Filesystem error: {0}")]
    Fs(#[from] FsError),

    #[error("Could not parse repository name from URL")]
    InvalidRepoUrl,

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Service for cloning git repositories and setting up zellij sessions
pub struct CloneService<Z, X, F, G>
where
    Z: ZellijOperations,
    X: ZoxideOperations,
    F: FsOperations,
    G: Git,
{
    zellij: Z,
    zoxide: X,
    fs: F,
    git: G,
}

impl<Z, X, F, G> CloneService<Z, X, F, G>
where
    Z: ZellijOperations,
    X: ZoxideOperations,
    F: FsOperations,
    G: Git,
{
    pub fn new(zellij: Z, zoxide: X, fs: F, git: G) -> Self {
        Self {
            zellij,
            zoxide,
            fs,
            git,
        }
    }

    /// Clone a git repository and create a zellij session for it
    pub fn clone_repo(
        &self,
        repo_url: &str,
        name: Option<&str>,
        path: Option<&PathBuf>,
        zellij_options: &ZellijOptions,
    ) -> Result<(), CloneError> {
        let repo_name = extract_repo_name(repo_url)?;
        let session_name = name.unwrap_or(repo_name);

        // Determine the parent directory
        let parent_dir = if let Some(p) = path {
            p.clone()
        } else {
            self.fs.current_dir()?
        };

        let clone_path = parent_dir.join(repo_name);
        let parent_dir_str = parent_dir
            .to_str()
            .ok_or_else(|| CloneError::InvalidPath(parent_dir.display().to_string()))?;

        // Clone using the git trait abstraction
        println!("Cloning {} into {}...", repo_url, clone_path.display());
        self.git.clone(repo_url, parent_dir_str, repo_name)?;

        println!(
            "Creating new session '{}' at {}",
            session_name,
            clone_path.display()
        );

        // Change to the cloned directory
        self.fs.set_current_dir(&clone_path)?;

        // Create new session
        self.zellij.new_session(session_name, zellij_options)?;

        // Add to zoxide database
        self.zoxide.add(&clone_path)?;

        Ok(())
    }
}

/// Extract repository name from URL
pub fn extract_repo_name(url: &str) -> Result<&str, CloneError> {
    let url = url.trim_end_matches(".git");
    url.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(CloneError::InvalidRepoUrl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::tests::MockFs;
    use std::path::{Path, PathBuf};
    use zellij_rs::{MockZellijClient, ZellijError};
    use zesh_git::GitError;
    use zox_rs::{MockZoxideClient, ZoxideError};

    // Test Git mock for clone tests
    struct TestGit {
        should_fail: bool,
    }

    impl TestGit {
        fn success() -> Self {
            Self { should_fail: false }
        }

        fn failing() -> Self {
            Self { should_fail: true }
        }
    }

    impl Git for TestGit {
        fn show_top_level(&self, _name: &str) -> Result<(bool, String), GitError> {
            Ok((false, String::new()))
        }

        fn git_common_dir(&self, _name: &str) -> Result<(bool, String), GitError> {
            Ok((false, String::new()))
        }

        fn clone(&self, _url: &str, _cmd_dir: &str, _dir: &str) -> Result<String, GitError> {
            if self.should_fail {
                Err(GitError::CommandError("clone failed".to_string()))
            } else {
                Ok("Clone successful".to_string())
            }
        }
    }

    fn create_service(
        git: TestGit,
    ) -> CloneService<MockZellijClient, MockZoxideClient, MockFs, TestGit> {
        let zellij = MockZellijClient::new();
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();
        CloneService::new(zellij, zoxide, fs, git)
    }

    #[test]
    fn test_extract_repo_name_https() {
        let name = extract_repo_name("https://github.com/user/my-repo.git").unwrap();
        assert_eq!(name, "my-repo");
    }

    #[test]
    fn test_extract_repo_name_https_no_git_suffix() {
        let name = extract_repo_name("https://github.com/user/my-repo").unwrap();
        assert_eq!(name, "my-repo");
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        let name = extract_repo_name("git@github.com:user/my-repo.git").unwrap();
        assert_eq!(name, "my-repo");
    }

    #[test]
    fn test_extract_repo_name_trailing_slash() {
        // Trailing slash after stripping .git leaves empty last segment
        let result = extract_repo_name("/");
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_repo_success() {
        let service = create_service(TestGit::success());

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            None,
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_ok());

        // Verify session was created
        let sessions = service.zellij.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "my-repo");
    }

    #[test]
    fn test_clone_repo_with_custom_name() {
        let service = create_service(TestGit::success());

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            Some("custom-session"),
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_ok());

        // Verify session was created with custom name
        let sessions = service.zellij.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "custom-session");
    }

    #[test]
    fn test_clone_repo_uses_current_dir_when_no_path() {
        let service = create_service(TestGit::success());

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            None,
            None,
            &ZellijOptions::default(),
        );

        assert!(result.is_ok());

        // Verify session was created
        let sessions = service.zellij.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "my-repo");
    }

    #[test]
    fn test_clone_repo_git_failure() {
        let service = create_service(TestGit::failing());

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            None,
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CloneError::Git(_))));

        // Verify no session was created
        let sessions = service.zellij.list_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_clone_repo_invalid_url() {
        let service = create_service(TestGit::success());

        let result = service.clone_repo(
            "/",
            None,
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CloneError::InvalidRepoUrl)));
    }

    #[test]
    fn test_clone_repo_zellij_failure() {
        // Use a failing zellij client
        struct FailingZellijClient;
        impl ZellijOperations for FailingZellijClient {
            fn list_sessions(&self) -> zellij_rs::ZellijResult<Vec<zellij_rs::Session>> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn attach_session(&self, _: &str) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn new_session(&self, _: &str, _: &ZellijOptions) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn kill_session(&self, _: &str) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn list_tabs(&self) -> zellij_rs::ZellijResult<Vec<zellij_rs::Tab>> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn new_tab(&self, _: Option<&str>) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn rename_tab(&self, _: &str) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn close_tab(&self) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
            fn run_command(&self, _: &str, _: &[&str]) -> zellij_rs::ZellijResult<()> {
                Err(ZellijError::CommandExecution("Command failed".to_string()))
            }
        }

        let service = CloneService::new(
            FailingZellijClient,
            MockZoxideClient::new(),
            MockFs::new(),
            TestGit::success(),
        );

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            None,
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CloneError::Zellij(_))));
    }

    #[test]
    fn test_clone_repo_zoxide_failure() {
        struct FailingZoxideClient;
        impl ZoxideOperations for FailingZoxideClient {
            fn add<P: AsRef<Path>>(&self, _: P) -> zox_rs::ZoxideResult<()> {
                Err(ZoxideError::CommandExecution("Command failed".to_string()))
            }
            fn list(&self) -> zox_rs::ZoxideResult<Vec<zox_rs::ZoxideEntry>> {
                Err(ZoxideError::CommandExecution("Command failed".to_string()))
            }
            fn query(&self, _: &[&str]) -> zox_rs::ZoxideResult<Vec<zox_rs::ZoxideEntry>> {
                Err(ZoxideError::CommandExecution("Command failed".to_string()))
            }
        }

        let service = CloneService::new(
            MockZellijClient::new(),
            FailingZoxideClient,
            MockFs::new(),
            TestGit::success(),
        );

        let result = service.clone_repo(
            "https://github.com/user/my-repo.git",
            None,
            Some(&PathBuf::from("/mock/parent")),
            &ZellijOptions::default(),
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CloneError::Zoxide(_))));
    }
}
