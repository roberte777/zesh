use std::path::{Path, PathBuf};
use thiserror::Error;
use zesh_git::{Git, GitError};

use crate::fs::{FsError, FsOperations};
use zellij_rs::{Session, ZellijError, ZellijOperations, options::ZellijOptions};
use zox_rs::{ZoxideError, ZoxideOperations};

// Update ConnectError to include Git errors
#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("Zellij error: {0}")]
    Zellij(#[from] ZellijError),

    #[error("Zoxide error: {0}")]
    Zoxide(#[from] ZoxideError),

    #[error("Filesystem error: {0}")]
    Fs(#[from] FsError),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("No matching sessions or directories found for '{0}'")]
    NoMatch(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Connect service handles connecting to zellij sessions, directories, or zoxide entries
pub struct ConnectService<Z, X, F, G>
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

impl<Z, X, F, G> ConnectService<Z, X, F, G>
where
    Z: ZellijOperations,
    X: ZoxideOperations,
    F: FsOperations,
    G: Git,
{
    /// Create a new ConnectService
    pub fn new(zellij: Z, zoxide: X, fs: F, git: G) -> Self {
        Self {
            zellij,
            zoxide,
            fs,
            git,
        }
    }

    /// Connect to a session by name, or a directory by path or zoxide query
    pub fn connect(&self, name: &str, options: &ZellijOptions) -> Result<(), ConnectError> {
        // First try to connect to an existing zellij session
        match self.connect_to_session(name) {
            Ok(_) => return Ok(()),
            Err(ConnectError::NoMatch(_)) => {}
            Err(e) => return Err(e),
        }

        // Then try if it's a directory path
        if let Ok(()) = self.connect_to_directory(name, options) {
            return Ok(());
        }

        // Finally try zoxide query
        self.connect_via_zoxide(name, options)
    }

    /// Connect to a session by name
    pub fn connect_to_session(&self, name: &str) -> Result<(), ConnectError> {
        let sessions = self.zellij.list_sessions()?;
        let session_match = sessions.iter().find(|s| s.name == name);

        if let Some(session) = session_match {
            self.zellij.attach_session(&session.name)?;
            Ok(())
        } else {
            Err(ConnectError::NoMatch(name.to_string()))
        }
    }

    /// Connect to a directory, creating a new session or attaching to an existing one
    pub fn connect_to_directory(
        &self,
        dir: &str,
        options: &ZellijOptions,
    ) -> Result<(), ConnectError> {
        let path = PathBuf::from(dir);

        // Validate and get canonical path
        let (canon_path, _) = self.fs.validate_dir_path(&path)?;

        // Get the session name based on Git repository info
        let session_name = self.get_session_name_for_path(&canon_path)?;

        // Check if session with this name already exists
        let sessions = self.zellij.list_sessions()?;
        let session_match = sessions.iter().find(|s| s.name == session_name);

        if let Some(session) = session_match {
            // If session exists, attach to it
            self.zellij.attach_session(&session.name)?;
        } else {
            // Otherwise create a new session
            self.fs.set_current_dir(&canon_path)?;
            self.zellij.new_session(&session_name, options)?;
        }

        // Add to zoxide database
        self.zoxide.add(&canon_path)?;

        Ok(())
    }

    /// Connect to a directory using zoxide query
    pub fn connect_via_zoxide(
        &self,
        query: &str,
        options: &ZellijOptions,
    ) -> Result<(), ConnectError> {
        let entries = self.zoxide.query(&[query])?;

        if entries.is_empty() {
            return Err(ConnectError::NoMatch(query.to_string()));
        }

        // Use the highest scored match
        let best_match = &entries[0];
        let path = &best_match.path;

        // Get the session name based on Git repository info
        let session_name = self.get_session_name_for_path(path)?;

        // Check if session with this name already exists
        let sessions = self.zellij.list_sessions()?;

        if sessions.iter().any(|s| s.name == session_name) {
            self.zellij.attach_session(&session_name)?;
            return Ok(());
        }

        // Create a new session
        self.fs.set_current_dir(path)?;
        self.zellij.new_session(&session_name, options)?;

        // Add to zoxide database
        self.zoxide.add(path)?;

        Ok(())
    }

    /// Get a list of active sessions
    pub fn list_sessions(&self) -> Result<Vec<Session>, ConnectError> {
        Ok(self.zellij.list_sessions()?)
    }

    /// Determine a session name for the given path, checking if it's in a Git repository
    fn get_session_name_for_path(&self, path: &Path) -> Result<String, ConnectError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| ConnectError::Other("Invalid path".to_string()))?;

        // Try to get the Git repository root
        match self.git.show_top_level(path_str) {
            Ok((true, git_root)) => {
                // Path is in a Git repository
                let git_root_path = PathBuf::from(&git_root);
                let git_root_name = self.fs.get_dir_name(&git_root_path)?;

                // If the path is the git root, just use the root name
                if path == git_root_path {
                    return Ok(git_root_name);
                }

                // Get the relative path from the Git root
                match path.strip_prefix(&git_root_path) {
                    Ok(rel_path) => {
                        if rel_path == Path::new("") {
                            // We're at the git root itself
                            Ok(git_root_name)
                        } else {
                            // We're in a subdirectory
                            // We have to use '_' because zellij does not
                            // support '/' in session names
                            Ok(format!("{}_{}", git_root_name, rel_path.display()))
                        }
                    }
                    Err(_) => Ok(self.fs.get_dir_name(path)?), // Fallback to dir name on error
                }
            }
            Ok((false, _)) => {
                // Not in a Git repository, just use the directory name
                Ok(self.fs.get_dir_name(path)?)
            }
            Err(e) => {
                // Error running git command, log it and fall back to directory name
                eprintln!("Git error: {}", e);
                Ok(self.fs.get_dir_name(path)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::tests::MockFs;
    use std::path::PathBuf;
    use std::{collections::HashMap, path::Path};
    use zellij_rs::{MockZellijClient, Session, ZellijError};
    use zox_rs::{MockZoxideClient, ZoxideEntry, ZoxideError};

    // Helper function to create a ConnectService with custom mocks
    fn create_service(
        zellij_sessions: Option<HashMap<String, bool>>,
        zoxide_paths: Option<HashMap<PathBuf, f64>>,
        fs_dirs: Option<Vec<(PathBuf, String)>>,
    ) -> ConnectService<MockZellijClient, MockZoxideClient, MockFs, TestGit> {
        // Setup mock zellij client
        let zellij = if let Some(sessions) = zellij_sessions {
            MockZellijClient::with_sessions(sessions)
        } else {
            MockZellijClient::new()
        };

        // Setup mock zoxide client
        let zoxide = if let Some(paths) = zoxide_paths {
            MockZoxideClient::with_paths(paths)
        } else {
            MockZoxideClient::new()
        };

        // Setup mock filesystem
        let fs = MockFs::new();
        if let Some(dirs) = fs_dirs {
            for (path, name) in dirs {
                fs.with_directory(&path, &name);
            }
        }

        ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"))
    }

    // Helper function to create a failing zellij client
    struct FailingZellijClient;
    impl ZellijOperations for FailingZellijClient {
        fn list_sessions(&self) -> zellij_rs::ZellijResult<Vec<Session>> {
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

    // Helper function to create a failing zoxide client
    struct FailingZoxideClient;
    impl ZoxideOperations for FailingZoxideClient {
        fn add<P: AsRef<Path>>(&self, _: P) -> zox_rs::ZoxideResult<()> {
            Err(ZoxideError::CommandExecution("Command failed".to_string()))
        }

        fn list(&self) -> zox_rs::ZoxideResult<Vec<ZoxideEntry>> {
            Err(ZoxideError::CommandExecution("Command failed".to_string()))
        }

        fn query(&self, _: &[&str]) -> zox_rs::ZoxideResult<Vec<ZoxideEntry>> {
            Err(ZoxideError::CommandExecution("Command failed".to_string()))
        }
    }

    // Helper function to create a failing filesystem
    struct FailingFs;
    impl FsOperations for FailingFs {
        fn exists(&self, _: &Path) -> bool {
            false
        }

        fn is_dir(&self, _: &Path) -> bool {
            false
        }

        fn canonicalize(&self, _: &Path) -> Result<PathBuf, FsError> {
            Err(FsError::Canonicalize(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to canonicalize",
            )))
        }

        fn get_dir_name(&self, path: &Path) -> Result<String, FsError> {
            Err(FsError::NoDirectoryName(path.display().to_string()))
        }

        fn set_current_dir(&self, _: &Path) -> Result<(), FsError> {
            Err(FsError::Other("Failed to set current dir".to_string()))
        }

        fn current_dir(&self) -> Result<PathBuf, FsError> {
            Err(FsError::Other("Failed to get current dir".to_string()))
        }
    }

    #[test]
    fn test_connect_to_session_basic() {
        // Setup service with two sessions
        let mut sessions = HashMap::new();
        sessions.insert("test-session".to_string(), false);
        sessions.insert("another-session".to_string(), true);
        let service = create_service(Some(sessions), None, None);

        // Test connecting to an existing session
        let result = service.connect_to_session("test-session");
        assert!(result.is_ok());

        // Verify that session is now marked as current
        let updated_sessions = service.list_sessions().unwrap();
        let session = updated_sessions
            .iter()
            .find(|s| s.name == "test-session")
            .unwrap();
        assert!(session.is_current);

        // Test connecting to non-existent session
        let result = service.connect_to_session("non-existent");
        assert!(result.is_err());
        if let Err(ConnectError::NoMatch(name)) = result {
            assert_eq!(name, "non-existent");
        } else {
            panic!("Expected ConnectError::NoMatch");
        }
    }

    #[test]
    fn test_connect_to_session_error_handling() {
        // Test with failing zellij client
        let zellij = FailingZellijClient;
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect_to_session("any-session");
        assert!(result.is_err());
        if let Err(ConnectError::Zellij(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zellij");
        }
    }

    #[test]
    fn test_connect_to_directory_new_session() {
        // Setup test directory
        let dir_path = PathBuf::from("/mock/project");
        let service = create_service(
            None,
            None,
            Some(vec![(dir_path.clone(), "project".to_string())]),
        );

        // Test connecting to directory that doesn't have a session yet
        let result = service.connect_to_directory("/mock/project", &ZellijOptions::default());
        assert!(result.is_ok());

        // After connection, should have a new session with the directory name
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "project");
        assert!(sessions[0].is_current);
    }

    #[test]
    fn test_connect_to_directory_existing_session() {
        // Setup test directory and existing session with same name
        let dir_path = PathBuf::from("/mock/project");
        let mut sessions = HashMap::new();
        sessions.insert("project".to_string(), false);

        let service = create_service(
            Some(sessions),
            None,
            Some(vec![(dir_path.clone(), "project".to_string())]),
        );

        // Test connecting to directory that already has a session
        let result = service.connect_to_directory("/mock/project", &ZellijOptions::default());
        assert!(result.is_ok());

        // After connection, should attach to existing session
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "project");
        assert!(sessions[0].is_current);
    }

    #[test]
    fn test_connect_to_directory_invalid_path() {
        let service = create_service(None, None, None);

        // Test with non-existent path
        let result = service.connect_to_directory("/mock/non-existent", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Fs(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Fs");
        }

        // Setup a file path (not a directory)
        let fs = MockFs::new();
        fs.with_file(&PathBuf::from("/mock/file.txt"));
        let service = ConnectService::new(
            MockZellijClient::new(),
            MockZoxideClient::new(),
            fs,
            TestGit::new(false, "./"),
        );

        // Test with a file path instead of directory
        let result = service.connect_to_directory("/mock/file.txt", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Fs(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Fs");
        }
    }

    #[test]
    fn test_connect_to_directory_error_handling() {
        // Setup test with failing filesystem
        let zellij = MockZellijClient::new();
        let zoxide = MockZoxideClient::new();
        let fs = FailingFs;
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect_to_directory("/any/path", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Fs(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Fs");
        }

        // Setup test with working filesystem but failing zellij
        let zellij = FailingZellijClient;
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();
        fs.with_directory(&PathBuf::from("/mock/project"), "project");
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect_to_directory("/mock/project", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Zellij(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zellij");
        }

        // Setup test with working filesystem and zellij but failing zoxide
        let zellij = MockZellijClient::new();
        let zoxide = FailingZoxideClient;
        let fs = MockFs::new();
        fs.with_directory(&PathBuf::from("/mock/project"), "project");
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect_to_directory("/mock/project", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Zoxide(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zoxide");
        }
    }

    #[test]
    fn test_connect_to_directory_with_special_chars() {
        // Test directory with spaces and special characters
        let dir_path = PathBuf::from("/mock/special project-name!");
        let service = create_service(
            None,
            None,
            Some(vec![(
                dir_path.clone(),
                "special project-name!".to_string(),
            )]),
        );

        let result =
            service.connect_to_directory("/mock/special project-name!", &ZellijOptions::default());
        assert!(result.is_ok());

        // Verify session was created with special characters
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "special project-name!");
    }

    #[test]
    fn test_connect_via_zoxide_single_match() {
        // Setup zoxide with a single matching path
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-dir"), 10.0);

        let service = create_service(
            None,
            Some(path_scores),
            Some(vec![(
                PathBuf::from("/mock/zoxide-dir"),
                "zoxide-dir".to_string(),
            )]),
        );

        // Test connecting via zoxide query that matches single entry
        let result = service.connect_via_zoxide("zoxide", &ZellijOptions::default());
        assert!(result.is_ok());

        // After connection, should have a new session with the directory name
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "zoxide-dir");
    }

    #[test]
    fn test_connect_via_zoxide_multiple_matches() {
        // Setup zoxide with multiple matching paths
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/best-match"), 20.0);
        path_scores.insert(PathBuf::from("/mock/second-match"), 10.0);

        let service = create_service(
            None,
            Some(path_scores),
            Some(vec![
                (PathBuf::from("/mock/best-match"), "best-match".to_string()),
                (
                    PathBuf::from("/mock/second-match"),
                    "second-match".to_string(),
                ),
            ]),
        );

        // Test connecting via zoxide query that matches multiple entries
        let result = service.connect_via_zoxide("match", &ZellijOptions::default());
        assert!(result.is_ok());

        // Should connect to highest scored match
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "best-match");
    }

    #[test]
    fn test_connect_via_zoxide_existing_session() {
        // Setup zoxide path and existing session with same name
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/existing"), 10.0);

        let mut sessions = HashMap::new();
        sessions.insert("existing".to_string(), false);

        let service = create_service(
            Some(sessions),
            Some(path_scores),
            Some(vec![(
                PathBuf::from("/mock/existing"),
                "existing".to_string(),
            )]),
        );

        // Test connecting via zoxide when session already exists
        let result = service.connect_via_zoxide("existing", &ZellijOptions::default());
        assert!(result.is_ok());

        // Should attach to existing session
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "existing");
        assert!(sessions[0].is_current);
    }

    #[test]
    fn test_connect_via_zoxide_no_matches() {
        // Setup empty zoxide database
        let service = create_service(None, None, None);

        // Test connecting via zoxide with no matches
        let result = service.connect_via_zoxide("non-existent", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::NoMatch(query)) = result {
            assert_eq!(query, "non-existent");
        } else {
            panic!("Expected ConnectError::NoMatch");
        }
    }

    #[test]
    fn test_connect_via_zoxide_error_handling() {
        // Setup failing zoxide client
        let zellij = MockZellijClient::new();
        let zoxide = FailingZoxideClient;
        let fs = MockFs::new();
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect_via_zoxide("query", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Zoxide(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zoxide");
        }
    }

    #[test]
    fn test_connect_success_scenarios() {
        // Setup for various connection types
        let mut sessions = HashMap::new();
        sessions.insert("existing-session".to_string(), false);

        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-match"), 10.0);

        let service = create_service(
            Some(sessions),
            Some(path_scores),
            Some(vec![
                (PathBuf::from("/mock/dir-path"), "dir-path".to_string()),
                (
                    PathBuf::from("/mock/zoxide-match"),
                    "zoxide-match".to_string(),
                ),
            ]),
        );

        // 1. Test connect to existing session
        let result = service.connect("existing-session", &ZellijOptions::default());
        assert!(result.is_ok());
        let sessions = service.list_sessions().unwrap();
        assert!(
            sessions
                .iter()
                .any(|s| s.name == "existing-session" && s.is_current)
        );

        // 2. Test connect to directory path
        let result = service.connect("/mock/dir-path", &ZellijOptions::default());
        assert!(result.is_ok());
        let sessions = service.list_sessions().unwrap();
        assert!(
            sessions
                .iter()
                .any(|s| s.name == "dir-path" && s.is_current)
        );

        // 3. Test connect via zoxide query
        let result = service.connect("zoxide-match", &ZellijOptions::default());
        assert!(result.is_ok());
        let sessions = service.list_sessions().unwrap();
        assert!(
            sessions
                .iter()
                .any(|s| s.name == "zoxide-match" && s.is_current)
        );
    }

    #[test]
    fn test_connect_fallback_behavior() {
        // Setup with no existing sessions but a valid directory
        let service = create_service(
            None,
            None,
            Some(vec![(
                PathBuf::from("/mock/valid-dir"),
                "valid-dir".to_string(),
            )]),
        );

        // Test with a name that's not a session, should fallback to directory path
        let result = service.connect("/mock/valid-dir", &ZellijOptions::default());
        assert!(result.is_ok());

        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "valid-dir");

        // Setup with no sessions but a zoxide match
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-path"), 10.0);

        let service = create_service(
            None,
            Some(path_scores),
            Some(vec![(
                PathBuf::from("/mock/zoxide-path"),
                "zoxide-path".to_string(),
            )]),
        );

        // Test with a name that should match zoxide query
        let result = service.connect("zoxide", &ZellijOptions::default());
        assert!(result.is_ok());

        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "zoxide-path");
    }

    #[test]
    fn test_connect_failure_cases() {
        // Setup with nothing valid
        let service = create_service(None, None, None);

        // Test with non-existent name
        let result = service.connect("non-existent", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::NoMatch(name)) = result {
            assert_eq!(name, "non-existent");
        } else {
            panic!("Expected ConnectError::NoMatch");
        }

        // Test with failing dependencies
        let zellij = FailingZellijClient;
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        let result = service.connect("anything", &ZellijOptions::default());
        assert!(result.is_err());
        if let Err(ConnectError::Zellij(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zellij");
        }
    }

    #[test]
    fn test_connect_case_sensitivity() {
        // Setup with case-sensitive session names
        let mut sessions = HashMap::new();
        sessions.insert("Case-Sensitive".to_string(), false);

        let service = create_service(Some(sessions), None, None);

        // Test with exact case match
        let result = service.connect("Case-Sensitive", &ZellijOptions::default());
        assert!(result.is_ok());

        // Test with different case (should fail)
        let result = service.connect("case-sensitive", &ZellijOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_sessions_error_handling() {
        // Setup with failing zellij
        let zellij = FailingZellijClient;
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();
        let service = ConnectService::new(zellij, zoxide, fs, TestGit::new(false, "./"));

        // Test listing sessions with failing dependency
        let result = service.list_sessions();
        assert!(result.is_err());
        if let Err(ConnectError::Zellij(_)) = result {
            // Expected error
        } else {
            panic!("Expected ConnectError::Zellij");
        }
    }

    #[test]
    fn test_complex_workflow() {
        // Setup for a complex workflow test
        let mut sessions = HashMap::new();
        sessions.insert("existing".to_string(), true);

        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/project1"), 10.0);
        path_scores.insert(PathBuf::from("/mock/project2"), 5.0);

        let service = create_service(
            Some(sessions.clone()),
            Some(path_scores.clone()),
            Some(vec![
                (PathBuf::from("/mock/project1"), "project1".to_string()),
                (PathBuf::from("/mock/project2"), "project2".to_string()),
                (PathBuf::from("/mock/project3"), "project3".to_string()),
            ]),
        );

        // 1. List initial sessions
        let initial_sessions = service.list_sessions().unwrap();
        assert_eq!(initial_sessions.len(), 1);
        assert_eq!(initial_sessions[0].name, "existing");

        // 2. Connect to directory directly
        let result = service.connect_to_directory("/mock/project3", &ZellijOptions::default());
        assert!(result.is_ok());

        // Verify new session created
        let sessions_after_dir = service.list_sessions().unwrap();
        assert_eq!(sessions_after_dir.len(), 2);
        assert!(
            sessions_after_dir
                .iter()
                .any(|s| s.name == "project3" && s.is_current)
        );
        assert!(
            sessions_after_dir
                .iter()
                .any(|s| s.name == "existing" && !s.is_current)
        );

        // 3. Connect via zoxide
        let result = service.connect_via_zoxide("project1", &ZellijOptions::default());
        assert!(result.is_ok());

        // Verify another session created
        let sessions_after_zoxide = service.list_sessions().unwrap();
        assert_eq!(sessions_after_zoxide.len(), 3);
        assert!(
            sessions_after_zoxide
                .iter()
                .any(|s| s.name == "project1" && s.is_current)
        );

        // 4. Connect back to first session
        let result = service.connect_to_session("existing");
        assert!(result.is_ok());

        // Verify attached to existing session
        let final_sessions = service.list_sessions().unwrap();
        assert_eq!(final_sessions.len(), 3);
        assert!(
            final_sessions
                .iter()
                .any(|s| s.name == "existing" && s.is_current)
        );
        assert!(
            final_sessions
                .iter()
                .any(|s| s.name == "project1" && !s.is_current)
        );
        assert!(
            final_sessions
                .iter()
                .any(|s| s.name == "project3" && !s.is_current)
        );
    }
    // Custom Git mock for testing
    struct TestGit {
        is_git_repo: bool,
        git_root: String,
    }

    impl TestGit {
        fn new(is_git_repo: bool, git_root: &str) -> Self {
            Self {
                is_git_repo,
                git_root: git_root.to_string(),
            }
        }
    }

    impl Git for TestGit {
        fn show_top_level(&self, _name: &str) -> Result<(bool, String), GitError> {
            Ok((self.is_git_repo, self.git_root.clone()))
        }

        fn git_common_dir(&self, _name: &str) -> Result<(bool, String), GitError> {
            Ok((self.is_git_repo, "/mock/repo/common-dir".to_string()))
        }

        fn clone(&self, _url: &str, _cmd_dir: &str, _dir: &str) -> Result<String, GitError> {
            Ok("Mock clone successful".to_string())
        }
    }

    // Helper function to create a ConnectService with the TestGit
    fn create_service_with_git(
        zellij_sessions: Option<HashMap<String, bool>>,
        zoxide_paths: Option<HashMap<PathBuf, f64>>,
        fs_dirs: Option<Vec<(PathBuf, String)>>,
        is_git_repo: bool,
        git_root: &str,
    ) -> ConnectService<MockZellijClient, MockZoxideClient, MockFs, TestGit> {
        let zellij = if let Some(sessions) = zellij_sessions {
            MockZellijClient::with_sessions(sessions)
        } else {
            MockZellijClient::new()
        };

        let zoxide = if let Some(paths) = zoxide_paths {
            MockZoxideClient::with_paths(paths)
        } else {
            MockZoxideClient::new()
        };

        let fs = MockFs::new();
        if let Some(dirs) = fs_dirs {
            for (path, name) in dirs {
                fs.with_directory(&path, &name);
            }
        }

        let git = TestGit::new(is_git_repo, git_root);

        ConnectService::new(zellij, zoxide, fs, git)
    }

    #[test]
    fn test_get_session_name_for_git_repo() {
        // Set up mock file system with git repo structure
        let git_root = PathBuf::from("/mock/foo");
        let subdir = PathBuf::from("/mock/foo/bar");

        let fs_dirs = vec![
            (git_root.clone(), "foo".to_string()),
            (subdir.clone(), "bar".to_string()),
        ];

        // Create service with git repo configuration
        let service = create_service_with_git(None, None, Some(fs_dirs), true, "/mock/foo");

        // Test getting session name for git root
        let name = service.get_session_name_for_path(&git_root).unwrap();
        assert_eq!(name, "foo");

        // Test getting session name for subdirectory
        let name = service.get_session_name_for_path(&subdir).unwrap();
        assert_eq!(name, "foo_bar");
    }

    #[test]
    fn test_get_session_name_for_non_git_path() {
        // Set up mock file system with directory not in git repo
        let path = PathBuf::from("/mock/not-git");

        let fs_dirs = vec![(path.clone(), "not-git".to_string())];

        // Create service with non-git configuration
        let service = create_service_with_git(None, None, Some(fs_dirs), false, "");

        // Test getting session name for non-git path
        let name = service.get_session_name_for_path(&path).unwrap();
        assert_eq!(name, "not-git");
    }

    #[test]
    fn test_connect_to_git_directory() {
        // Set up mock file system with git repo structure
        let git_root = PathBuf::from("/mock/project");
        let subdir = PathBuf::from("/mock/project/feature");

        let fs_dirs = vec![
            (git_root.clone(), "project".to_string()),
            (subdir.clone(), "feature".to_string()),
        ];

        // Create service with git repo configuration
        let service = create_service_with_git(None, None, Some(fs_dirs), true, "/mock/project");

        // Connect to subdirectory
        let result =
            service.connect_to_directory("/mock/project/feature", &ZellijOptions::default());
        assert!(result.is_ok());

        // Verify the session name is git-aware
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "project_feature");
        assert!(sessions[0].is_current);
    }

    #[test]
    fn test_connect_to_directory_with_options() {
        // Setup test directory
        let dir_path = PathBuf::from("/mock/project");
        let service = create_service(
            None,
            None,
            Some(vec![(dir_path.clone(), "project".to_string())]),
        );

        // Create options with debug enabled
        let options = ZellijOptions {
            new_session_with_layout: None,
            config: None,
            config_dir: None,
            data_dir: None,
            max_panes: None,
            debug: true,
        };

        // Test connecting to directory with options
        let result = service.connect_to_directory("/mock/project", &options);
        assert!(result.is_ok());

        // After connection, should have a new session with the directory name
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "project");
        assert!(sessions[0].is_current);
    }

    #[test]
    fn test_connect_via_zoxide_with_options() {
        // Setup zoxide with a single matching path
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-dir"), 10.0);

        let service = create_service(
            None,
            Some(path_scores),
            Some(vec![(
                PathBuf::from("/mock/zoxide-dir"),
                "zoxide-dir".to_string(),
            )]),
        );

        // Create options with custom config
        let options = ZellijOptions {
            new_session_with_layout: None,
            config: Some("custom-config.yaml".to_string()),
            config_dir: None,
            data_dir: None,
            max_panes: None,
            debug: false,
        };

        // Test connecting via zoxide with options
        let result = service.connect_via_zoxide("zoxide", &options);
        assert!(result.is_ok());

        // Should have a new session with the directory name
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "zoxide-dir");
        assert!(sessions[0].is_current);
    }
}
