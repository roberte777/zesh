use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

/// Result type for zoxide operations
pub type ZoxideResult<T> = Result<T, ZoxideError>;

/// Error type for zoxide operations
#[derive(Debug, thiserror::Error)]
pub enum ZoxideError {
    #[error("Failed to execute zoxide command: {0}")]
    CommandExecution(String),

    #[error("Failed to parse zoxide output: {0}")]
    OutputParsing(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// Entry with path and score from zoxide
#[derive(Debug, Clone, PartialEq)]
pub struct ZoxideEntry {
    pub path: PathBuf,
    pub score: f64,
}

/// Trait defining zoxide operations
pub trait ZoxideOperations {
    /// Add a path to zoxide database
    fn add<P: AsRef<Path>>(&self, path: P) -> ZoxideResult<()>;

    /// List all paths in zoxide database with their scores
    fn list(&self) -> ZoxideResult<Vec<ZoxideEntry>>;

    /// Query zoxide for matching paths
    fn query(&self, keywords: &[&str]) -> ZoxideResult<Vec<ZoxideEntry>>;
}

/// Default implementation that calls the real zoxide command
#[derive(Clone)]
pub struct ZoxideClient;

impl ZoxideClient {
    /// Create a new ZoxideClient
    pub fn new() -> Self {
        ZoxideClient
    }
}

impl Default for ZoxideClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoxideOperations for ZoxideClient {
    fn add<P: AsRef<Path>>(&self, path: P) -> ZoxideResult<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| ZoxideError::CommandExecution("Invalid path".to_string()))?;

        let output = Command::new("zoxide").arg("add").arg(path_str).output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZoxideError::CommandExecution(error.to_string()));
        }

        Ok(())
    }

    fn list(&self) -> ZoxideResult<Vec<ZoxideEntry>> {
        let output = Command::new("zoxide").arg("query").arg("--list").output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZoxideError::CommandExecution(error.to_string()));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        parse_zoxide_list_output(stdout)
    }

    fn query(&self, keywords: &[&str]) -> ZoxideResult<Vec<ZoxideEntry>> {
        let mut cmd = Command::new("zoxide");
        cmd.arg("query");

        // Add --score flag to get scores
        cmd.arg("--score");

        // Add all keywords
        for keyword in keywords {
            cmd.arg(keyword);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZoxideError::CommandExecution(error.to_string()));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        parse_zoxide_query_output(stdout)
    }
}

/// Parse output from zoxide query --list or zoxide query --score
fn parse_zoxide_list_output(output: &str) -> ZoxideResult<Vec<ZoxideEntry>> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(ZoxideError::OutputParsing(format!(
                "Invalid output format: {}",
                line
            )));
        }

        let score = parts[0].parse::<f64>().map_err(|_| {
            ZoxideError::OutputParsing(format!("Failed to parse score: {}", parts[0]))
        })?;

        let path = PathBuf::from(parts[1]);

        entries.push(ZoxideEntry { path, score });
    }

    Ok(entries)
}

/// Parse output from zoxide query with keywords and --score flag
fn parse_zoxide_query_output(output: &str) -> ZoxideResult<Vec<ZoxideEntry>> {
    // The output format is the same as list output when using --score
    parse_zoxide_list_output(output)
}

/// A mock implementation of ZoxideOperations for testing
#[derive(Default)]
pub struct MockZoxideClient {
    // Store paths and their scores
    paths: RefCell<HashMap<PathBuf, f64>>,
}

impl MockZoxideClient {
    pub fn new() -> Self {
        Self {
            paths: RefCell::new(HashMap::new()),
        }
    }

    /// Preset paths and scores for testing
    pub fn with_paths(paths: HashMap<PathBuf, f64>) -> Self {
        Self {
            paths: RefCell::new(paths),
        }
    }
}

impl ZoxideOperations for MockZoxideClient {
    fn add<P: AsRef<Path>>(&self, path: P) -> ZoxideResult<()> {
        let path_buf = path.as_ref().to_path_buf();
        let mut paths = self.paths.borrow_mut();

        // If path already exists, increase its score by 1
        // Otherwise add it with a score of 1
        *paths.entry(path_buf).or_insert(0.0) += 1.0;

        Ok(())
    }

    fn list(&self) -> ZoxideResult<Vec<ZoxideEntry>> {
        let paths = self.paths.borrow();

        let mut entries: Vec<ZoxideEntry> = paths
            .iter()
            .map(|(path, &score)| ZoxideEntry {
                path: path.clone(),
                score,
            })
            .collect();

        // Sort by score descending
        entries.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(entries)
    }

    fn query(&self, keywords: &[&str]) -> ZoxideResult<Vec<ZoxideEntry>> {
        let paths = self.paths.borrow();

        // Simple filtering: check if any keyword is a substring of the path
        let filtered: Vec<ZoxideEntry> = paths
            .iter()
            .filter(|(path, _)| {
                if keywords.is_empty() {
                    return true;
                }

                let path_str = path.to_string_lossy().to_lowercase();
                keywords
                    .iter()
                    .any(|&keyword| path_str.contains(&keyword.to_lowercase()))
            })
            .map(|(path, &score)| ZoxideEntry {
                path: path.clone(),
                score,
            })
            .collect();

        // Sort by score descending
        let mut result = filtered;
        result.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(result)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn test_mock_zoxide_add() {
        let client = MockZoxideClient::new();

        client.add("/home/user/projects").unwrap();
        client.add("/home/user/documents").unwrap();
        client.add("/home/user/projects").unwrap();

        let entries = client.list().unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, PathBuf::from("/home/user/projects"));
        assert_eq!(entries[0].score, 2.0);
        assert_eq!(entries[1].path, PathBuf::from("/home/user/documents"));
        assert_eq!(entries[1].score, 1.0);
    }

    #[test]
    fn test_mock_zoxide_query() {
        let mut paths = HashMap::new();
        paths.insert(PathBuf::from("/home/user/projects"), 10.0);
        paths.insert(PathBuf::from("/home/user/documents"), 5.0);
        paths.insert(PathBuf::from("/var/log"), 2.0);

        let client = MockZoxideClient::with_paths(paths);

        let results = client.query(&["user"]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, PathBuf::from("/home/user/projects"));
        assert_eq!(results[1].path, PathBuf::from("/home/user/documents"));
    }
}
