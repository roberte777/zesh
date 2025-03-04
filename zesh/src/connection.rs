use std::path::PathBuf;
use thiserror::Error;

use crate::fs::{FsError, FsOperations};
use zellij_rs::{Session, ZellijError, ZellijOperations};
use zox_rs::{ZoxideError, ZoxideOperations};

/// Error type for Connect service operations
#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("Zellij error: {0}")]
    Zellij(#[from] ZellijError),

    #[error("Zoxide error: {0}")]
    Zoxide(#[from] ZoxideError),

    #[error("Filesystem error: {0}")]
    Fs(#[from] FsError),

    #[error("No matching sessions or directories found for '{0}'")]
    NoMatch(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Connect service handles connecting to zellij sessions, directories, or zoxide entries
pub struct ConnectService<Z, X, F>
where
    Z: ZellijOperations,
    X: ZoxideOperations,
    F: FsOperations,
{
    zellij: Z,
    zoxide: X,
    fs: F,
}

impl<Z, X, F> ConnectService<Z, X, F>
where
    Z: ZellijOperations,
    X: ZoxideOperations,
    F: FsOperations,
{
    /// Create a new ConnectService
    pub fn new(zellij: Z, zoxide: X, fs: F) -> Self {
        Self { zellij, zoxide, fs }
    }

    /// Connect to a session by name, or a directory by path or zoxide query
    pub fn connect(&self, name: &str) -> Result<(), ConnectError> {
        // First try to connect to an existing zellij session
        if let Ok(()) = self.connect_to_session(name) {
            return Ok(());
        }

        // Then try if it's a directory path
        if let Ok(()) = self.connect_to_directory(name) {
            return Ok(());
        }

        // Finally try zoxide query
        self.connect_via_zoxide(name)
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
    pub fn connect_to_directory(&self, dir: &str) -> Result<(), ConnectError> {
        let path = PathBuf::from(dir);

        // Validate and get canonical path and directory name
        let (canon_path, dir_name) = self.fs.validate_dir_path(&path)?;

        // Check if session with this name already exists
        let sessions = self.zellij.list_sessions()?;
        let session_match = sessions.iter().find(|s| s.name == dir_name);

        if let Some(session) = session_match {
            // If session exists, attach to it
            self.zellij.attach_session(&session.name)?;
        } else {
            // Otherwise create a new session
            self.fs.set_current_dir(&canon_path)?;
            self.zellij.new_session(&dir_name)?;
        }

        // Add to zoxide database
        self.zoxide.add(canon_path)?;

        Ok(())
    }

    /// Connect to a directory using zoxide query
    pub fn connect_via_zoxide(&self, query: &str) -> Result<(), ConnectError> {
        let entries = self.zoxide.query(&[query])?;

        if entries.is_empty() {
            return Err(ConnectError::NoMatch(query.to_string()));
        }

        // Use the highest scored match
        let best_match = &entries[0];
        let path = &best_match.path;

        // Get directory name for session name
        let session_name = self.fs.get_dir_name(path)?;

        // Check if session with this name already exists
        let sessions = self.zellij.list_sessions()?;

        if sessions.iter().any(|s| s.name == session_name) {
            self.zellij.attach_session(&session_name)?;
            return Ok(());
        }

        // Create a new session
        self.fs.set_current_dir(path)?;
        self.zellij.new_session(&session_name)?;

        // Add to zoxide database
        self.zoxide.add(path)?;

        Ok(())
    }

    /// Get a list of active sessions
    pub fn list_sessions(&self) -> Result<Vec<Session>, ConnectError> {
        Ok(self.zellij.list_sessions()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::tests::MockFs;
    use std::collections::HashMap;
    use zellij_rs::MockZellijClient;
    use zox_rs::MockZoxideClient;

    #[test]
    fn test_connect_to_session() {
        // Set up mock dependencies
        let mut sessions = HashMap::new();
        sessions.insert("test-session".to_string(), false);

        let zellij = MockZellijClient::with_sessions(sessions);
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();

        let service = ConnectService::new(zellij, zoxide, fs);

        // Test connecting to an existing session
        let result = service.connect_to_session("test-session");
        assert!(result.is_ok());

        // Test connecting to non-existent session
        let result = service.connect_to_session("non-existent");
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_to_directory() {
        // Set up mock dependencies
        let zellij = MockZellijClient::new();
        let zoxide = MockZoxideClient::new();
        let fs = MockFs::new();

        // Set up a mock directory
        let dir_path = PathBuf::from("/mock/project");
        fs.with_directory(&dir_path, "project");

        let service = ConnectService::new(zellij, zoxide, fs);

        // Test connecting to directory
        let result = service.connect_to_directory("/mock/project");
        assert!(result.is_ok());

        // After connection, a new session should be created
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "project");

        // If we connect again, it should reuse the session
        let result = service.connect_to_directory("/mock/project");
        assert!(result.is_ok());
    }

    #[test]
    fn test_connect_via_zoxide() {
        // Set up mock dependencies
        let zellij = MockZellijClient::new();

        // Set up mock zoxide with an entry
        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-dir"), 10.0);
        let zoxide = MockZoxideClient::with_paths(path_scores);

        // Set up mock filesystem
        let fs = MockFs::new();
        fs.with_directory(&PathBuf::from("/mock/zoxide-dir"), "zoxide-dir");

        let service = ConnectService::new(zellij, zoxide, fs);

        // Test connecting via zoxide query
        let result = service.connect_via_zoxide("zoxide");
        assert!(result.is_ok());

        // After connection, a new session should be created
        let sessions = service.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "zoxide-dir");
    }

    #[test]
    fn test_connect() {
        // Set up mock dependencies
        let mut sessions = HashMap::new();
        sessions.insert("existing-session".to_string(), false);
        let zellij = MockZellijClient::with_sessions(sessions);

        let mut path_scores = HashMap::new();
        path_scores.insert(PathBuf::from("/mock/zoxide-project"), 10.0);
        let zoxide = MockZoxideClient::with_paths(path_scores);

        let fs = MockFs::new();
        fs.with_directory(&PathBuf::from("/mock/dir-project"), "dir-project");
        fs.with_directory(&PathBuf::from("/mock/zoxide-project"), "zoxide-project");

        let service = ConnectService::new(zellij, zoxide, fs);

        // Test connecting to existing session
        let result = service.connect("existing-session");
        assert!(result.is_ok());

        // Test connecting to directory path
        let result = service.connect("/mock/dir-project");
        assert!(result.is_ok());

        // Test connecting via zoxide query
        let result = service.connect("zoxide");
        assert!(result.is_ok());

        // Test connecting to non-existent target
        let result = service.connect("non-existent");
        assert!(result.is_err());
    }
}
