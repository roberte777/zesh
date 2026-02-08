use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error type for filesystem operations
#[derive(Debug, Error)]
pub enum FsError {
    #[error("Path doesn't exist: {0}")]
    PathNotFound(String),

    #[error("Path is not a directory: {0}")]
    NotADirectory(String),

    #[error("Failed to get directory name: {0}")]
    NoDirectoryName(String),

    #[error("Failed to canonicalize path: {0}")]
    Canonicalize(#[from] std::io::Error),

    #[error("Other filesystem error: {0}")]
    Other(String),
}

/// Trait for filesystem operations
pub trait FsOperations {
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Canonicalize a path (resolve symlinks, etc.)
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsError>;

    /// Get the directory name from a path
    fn get_dir_name(&self, path: &Path) -> Result<String, FsError>;

    /// Set the current directory
    fn set_current_dir(&self, path: &Path) -> Result<(), FsError>;

    /// Get the current directory
    fn current_dir(&self) -> Result<PathBuf, FsError>;

    /// Extract the directory name from a path and confirm it's a valid directory
    fn validate_dir_path(&self, path: &Path) -> Result<(PathBuf, String), FsError> {
        let canon_path = self.canonicalize(path)?;

        if !self.exists(&canon_path) {
            return Err(FsError::PathNotFound(canon_path.display().to_string()));
        }

        if !self.is_dir(&canon_path) {
            return Err(FsError::NotADirectory(canon_path.display().to_string()));
        }

        let name = self.get_dir_name(&canon_path)?;

        Ok((canon_path, name))
    }
}

/// Default implementation that uses the standard filesystem
#[derive(Copy, Clone)]
pub struct RealFs;

impl RealFs {
    /// Create a new RealFs
    pub fn new() -> Self {
        RealFs
    }
}

impl Default for RealFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FsOperations for RealFs {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsError> {
        path.canonicalize().map_err(FsError::Canonicalize)
    }

    fn get_dir_name(&self, path: &Path) -> Result<String, FsError> {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .ok_or_else(|| FsError::NoDirectoryName(path.display().to_string()))
    }

    fn set_current_dir(&self, path: &Path) -> Result<(), FsError> {
        std::env::set_current_dir(path).map_err(|e| FsError::Other(e.to_string()))
    }

    fn current_dir(&self) -> Result<PathBuf, FsError> {
        std::env::current_dir().map_err(|e| FsError::Other(e.to_string()))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// A mock implementation of filesystem operations for testing
    #[derive(Default)]
    pub struct MockFs {
        exists_map: RefCell<HashMap<PathBuf, bool>>,
        is_dir_map: RefCell<HashMap<PathBuf, bool>>,
        dir_names: RefCell<HashMap<PathBuf, String>>,
        current_dir: RefCell<PathBuf>,
    }

    impl MockFs {
        pub fn new() -> Self {
            Self {
                exists_map: RefCell::new(HashMap::new()),
                is_dir_map: RefCell::new(HashMap::new()),
                dir_names: RefCell::new(HashMap::new()),
                current_dir: RefCell::new(PathBuf::from("/mock/current")),
            }
        }

        pub fn with_directory(&self, path: &Path, dir_name: &str) -> &Self {
            let path_buf = path.to_path_buf();
            self.exists_map.borrow_mut().insert(path_buf.clone(), true);
            self.is_dir_map.borrow_mut().insert(path_buf.clone(), true);
            self.dir_names
                .borrow_mut()
                .insert(path_buf, dir_name.to_string());
            self
        }

        pub fn with_file(&self, path: &Path) -> &Self {
            let path_buf = path.to_path_buf();
            self.exists_map.borrow_mut().insert(path_buf.clone(), true);
            self.is_dir_map.borrow_mut().insert(path_buf, false);
            self
        }

        pub fn with_current_dir(&self, path: &Path) -> &Self {
            *self.current_dir.borrow_mut() = path.to_path_buf();
            self
        }
    }

    impl FsOperations for MockFs {
        fn exists(&self, path: &Path) -> bool {
            *self
                .exists_map
                .borrow()
                .get(&path.to_path_buf())
                .unwrap_or(&false)
        }

        fn is_dir(&self, path: &Path) -> bool {
            *self
                .is_dir_map
                .borrow()
                .get(&path.to_path_buf())
                .unwrap_or(&false)
        }

        fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsError> {
            // For mock, we just return the path as is
            Ok(path.to_path_buf())
        }

        fn get_dir_name(&self, path: &Path) -> Result<String, FsError> {
            let path_buf = path.to_path_buf();
            self.dir_names
                .borrow()
                .get(&path_buf)
                .cloned()
                .ok_or_else(|| FsError::NoDirectoryName(path.display().to_string()))
        }

        fn set_current_dir(&self, path: &Path) -> Result<(), FsError> {
            *self.current_dir.borrow_mut() = path.to_path_buf();
            Ok(())
        }

        fn current_dir(&self) -> Result<PathBuf, FsError> {
            Ok(self.current_dir.borrow().clone())
        }
    }

    #[test]
    fn test_validate_dir_path() {
        let mock_fs = MockFs::new();
        let dir_path = PathBuf::from("/mock/valid-dir");
        mock_fs.with_directory(&dir_path, "valid-dir");

        let result = mock_fs.validate_dir_path(&dir_path);
        assert!(result.is_ok());
        let (path, name) = result.unwrap();
        assert_eq!(path, dir_path);
        assert_eq!(name, "valid-dir");

        // Test with non-existent path
        let bad_path = PathBuf::from("/mock/non-existent");
        let result = mock_fs.validate_dir_path(&bad_path);
        assert!(result.is_err());

        // Test with file instead of directory
        let file_path = PathBuf::from("/mock/file.txt");
        mock_fs.with_file(&file_path);
        let result = mock_fs.validate_dir_path(&file_path);
        assert!(result.is_err());
    }
}
