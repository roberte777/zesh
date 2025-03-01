use std::io::Read;
use std::process::{Command, Stdio};
use std::str;

/// Result type for zellij operations
pub type ZellijResult<T> = Result<T, ZellijError>;

/// Error type for zellij operations
#[derive(Debug, thiserror::Error)]
pub enum ZellijError {
    #[error("Failed to execute zellij command: {0}")]
    CommandExecution(String),

    #[error("Failed to parse zellij output: {0}")]
    OutputParsing(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// Represents a Zellij session
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub is_current: bool,
}

/// Represents a Zellij pane
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    pub id: u32,
    pub name: Option<String>,
    pub is_focused: bool,
    pub is_plugin: bool,
}

/// Represents a Zellij tab
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub position: u32,
    pub name: Option<String>,
    pub is_active: bool,
    pub panes: Vec<Pane>,
}

/// Trait defining zellij operations
pub trait ZellijOperations {
    /// List all active sessions
    fn list_sessions(&self) -> ZellijResult<Vec<Session>>;

    /// Attach to an existing session
    fn attach_session(&self, session_name: &str) -> ZellijResult<()>;

    /// Create a new session
    fn new_session(&self, session_name: &str) -> ZellijResult<()>;

    /// Close a session
    fn kill_session(&self, session_name: &str) -> ZellijResult<()>;

    /// List all tabs in the current session
    fn list_tabs(&self) -> ZellijResult<Vec<Tab>>;

    /// Create a new tab with optional name
    fn new_tab(&self, name: Option<&str>) -> ZellijResult<()>;

    /// Rename the current tab
    fn rename_tab(&self, name: &str) -> ZellijResult<()>;

    /// Close the current tab
    fn close_tab(&self) -> ZellijResult<()>;

    /// Run a command in a new pane
    fn run_command(&self, command: &str, args: &[&str]) -> ZellijResult<()>;
}

/// Default implementation that calls the real zellij command
pub struct ZellijClient;

impl ZellijClient {
    /// Create a new ZellijClient
    pub fn new() -> Self {
        ZellijClient
    }
}

impl Default for ZellijClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ZellijOperations for ZellijClient {
    fn list_sessions(&self) -> ZellijResult<Vec<Session>> {
        let output = Command::new("zellij")
            .arg("list-sessions")
            .arg("--no-formatting")
            .output()?;

        // if there are no sessions, success will be false.
        let stdout = if !output.status.success() {
            ""
        } else {
            str::from_utf8(&output.stdout)?
        };

        parse_session_list(stdout)
    }

    fn attach_session(&self, session_name: &str) -> ZellijResult<()> {
        let mut child = Command::new("zellij")
            .arg("attach")
            .arg(session_name)
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stderr = String::new();

        if let Some(mut err) = child.stderr.take() {
            err.read_to_string(&mut stderr)?;
        }

        let status = child.wait()?;

        if !status.success() {
            return Err(ZellijError::CommandExecution(stderr));
        }

        Ok(())
    }

    fn new_session(&self, session_name: &str) -> ZellijResult<()> {
        let mut child = Command::new("zellij")
            .arg("--session")
            .arg(session_name)
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stderr = String::new();

        if let Some(mut err) = child.stderr.take() {
            err.read_to_string(&mut stderr)?;
        }

        let status = child.wait()?;

        if !status.success() {
            return Err(ZellijError::CommandExecution(stderr));
        }

        Ok(())
    }

    fn kill_session(&self, session_name: &str) -> ZellijResult<()> {
        let output = Command::new("zellij")
            .arg("kill-session")
            .arg(session_name)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        Ok(())
    }

    fn list_tabs(&self) -> ZellijResult<Vec<Tab>> {
        // This requires zellij 0.35.0+ for JSON output format
        let output = Command::new("zellij")
            .args(["action", "query", "--tabs"])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        parse_tabs_json(stdout)
    }

    fn new_tab(&self, name: Option<&str>) -> ZellijResult<()> {
        let mut cmd = Command::new("zellij");
        cmd.args(["action", "new-tab"]);

        if let Some(tab_name) = name {
            cmd.args(["--name", tab_name]);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        Ok(())
    }

    fn rename_tab(&self, name: &str) -> ZellijResult<()> {
        let output = Command::new("zellij")
            .args(["action", "rename-tab", name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        Ok(())
    }

    fn close_tab(&self) -> ZellijResult<()> {
        let output = Command::new("zellij")
            .args(["action", "close-tab"])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        Ok(())
    }

    fn run_command(&self, command: &str, args: &[&str]) -> ZellijResult<()> {
        let mut cmd = Command::new("zellij");
        cmd.arg("run");
        cmd.arg("--");
        cmd.arg(command);

        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ZellijError::CommandExecution(error.to_string()));
        }

        Ok(())
    }
}

/// Parse zellij list-sessions output
fn parse_session_list(output: &str) -> ZellijResult<Vec<Session>> {
    let mut sessions = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_current = line.contains("(current)");
        let name: String = line.splitn(2, ' ').collect::<Vec<&str>>()[0].to_string();

        sessions.push(Session { name, is_current });
    }

    Ok(sessions)
}

/// Parse zellij query --tabs JSON output
fn parse_tabs_json(_json: &str) -> ZellijResult<Vec<Tab>> {
    // Note: In a real implementation, you'd use serde_json here.
    // For simplicity, I'm using a simplified representation.
    // You should add serde and serde_json to your dependencies
    // and implement a proper JSON parser.

    // This is a placeholder for proper JSON parsing
    let tabs = Vec::new();

    // In a real implementation, you'd do something like:
    // let tabs: Vec<Tab> = serde_json::from_str(json)?;

    Ok(tabs)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// A mock implementation of ZellijOperations for testing
    #[derive(Default)]
    pub struct MockZellijClient {
        sessions: RefCell<HashMap<String, bool>>, // session_name -> is_current
        tabs: RefCell<Vec<Tab>>,
        current_session: RefCell<Option<String>>,
    }

    impl MockZellijClient {
        pub fn new() -> Self {
            Self {
                sessions: RefCell::new(HashMap::new()),
                tabs: RefCell::new(Vec::new()),
                current_session: RefCell::new(None),
            }
        }

        /// Preset sessions for testing
        pub fn with_sessions(sessions: HashMap<String, bool>) -> Self {
            let client = Self::new();
            *client.sessions.borrow_mut() = sessions.clone();

            // Set the first session that is current as the current session
            for (name, is_current) in sessions.iter() {
                if *is_current {
                    *client.current_session.borrow_mut() = Some(name.clone());
                    break;
                }
            }

            client
        }

        /// Preset tabs for testing
        pub fn with_tabs(tabs: Vec<Tab>) -> Self {
            let client = Self::new();
            *client.tabs.borrow_mut() = tabs;
            client
        }
    }

    impl ZellijOperations for MockZellijClient {
        fn list_sessions(&self) -> ZellijResult<Vec<Session>> {
            let sessions = self.sessions.borrow();
            let result = sessions
                .iter()
                .map(|(name, &is_current)| Session {
                    name: name.clone(),
                    is_current,
                })
                .collect();

            Ok(result)
        }

        fn attach_session(&self, session_name: &str) -> ZellijResult<()> {
            let mut sessions = self.sessions.borrow_mut();

            if !sessions.contains_key(session_name) {
                return Err(ZellijError::CommandExecution(format!(
                    "Session '{}' not found",
                    session_name
                )));
            }

            // Mark the current session as not current
            if let Some(current_session) = self.current_session.borrow().as_ref() {
                if let Some(session) = sessions.get_mut(current_session) {
                    *session = false;
                }
            }

            // Mark the new session as current
            if let Some(session) = sessions.get_mut(session_name) {
                *session = true;
                *self.current_session.borrow_mut() = Some(session_name.to_string());
            }

            Ok(())
        }

        fn new_session(&self, session_name: &str) -> ZellijResult<()> {
            let mut sessions = self.sessions.borrow_mut();

            // Mark the current session as not current
            if let Some(current_session) = self.current_session.borrow().as_ref() {
                if let Some(session) = sessions.get_mut(current_session) {
                    *session = false;
                }
            }

            // Add the new session and mark it as current
            sessions.insert(session_name.to_string(), true);
            *self.current_session.borrow_mut() = Some(session_name.to_string());

            Ok(())
        }

        fn kill_session(&self, session_name: &str) -> ZellijResult<()> {
            let mut sessions = self.sessions.borrow_mut();

            if !sessions.contains_key(session_name) {
                return Err(ZellijError::CommandExecution(format!(
                    "Session '{}' not found",
                    session_name
                )));
            }

            // Remove the session
            sessions.remove(session_name);

            // If we removed the current session, set current_session to None
            if let Some(current) = self.current_session.borrow().as_ref() {
                if current == session_name {
                    *self.current_session.borrow_mut() = None;
                }
            }

            Ok(())
        }

        fn list_tabs(&self) -> ZellijResult<Vec<Tab>> {
            Ok(self.tabs.borrow().clone())
        }

        fn new_tab(&self, name: Option<&str>) -> ZellijResult<()> {
            let mut tabs = self.tabs.borrow_mut();

            // Set all existing tabs to not active
            for tab in tabs.iter_mut() {
                tab.is_active = false;
            }

            // Create a new tab and set it as active
            let position = tabs.len() as u32;
            tabs.push(Tab {
                position,
                name: name.map(String::from),
                is_active: true,
                panes: Vec::new(),
            });

            Ok(())
        }

        fn rename_tab(&self, name: &str) -> ZellijResult<()> {
            let mut tabs = self.tabs.borrow_mut();

            // Find the active tab and rename it
            for tab in tabs.iter_mut() {
                if tab.is_active {
                    tab.name = Some(name.to_string());
                    return Ok(());
                }
            }

            Err(ZellijError::CommandExecution(
                "No active tab found".to_string(),
            ))
        }

        fn close_tab(&self) -> ZellijResult<()> {
            let mut tabs = self.tabs.borrow_mut();

            // Find the active tab
            let active_index = tabs.iter().position(|tab| tab.is_active);

            if let Some(index) = active_index {
                // Remove the active tab
                tabs.remove(index);

                let tab_len = tabs.len();

                // Update positions and set a new active tab if possible
                for (i, tab) in tabs.iter_mut().enumerate() {
                    tab.position = i as u32;
                    if i == index.min(tab_len - 1) {
                        tab.is_active = true;
                    }
                }

                Ok(())
            } else {
                Err(ZellijError::CommandExecution(
                    "No active tab found".to_string(),
                ))
            }
        }

        fn run_command(&self, _command: &str, _args: &[&str]) -> ZellijResult<()> {
            // In a mock, we don't actually run commands
            // Just pretend it succeeded
            Ok(())
        }
    }

    #[test]
    fn test_mock_zellij_sessions() {
        let mut sessions = HashMap::new();
        sessions.insert("work".to_string(), true);
        sessions.insert("personal".to_string(), false);

        let client = MockZellijClient::with_sessions(sessions);

        // Test listing sessions
        let listed_sessions = client.list_sessions().unwrap();
        assert_eq!(listed_sessions.len(), 2);

        // Find the current session
        let current_session = listed_sessions.iter().find(|s| s.is_current).unwrap();
        assert_eq!(current_session.name, "work");

        // Test creating a new session
        client.new_session("project").unwrap();
        let updated_sessions = client.list_sessions().unwrap();
        assert_eq!(updated_sessions.len(), 3);

        // Verify the new session is current
        let new_current = updated_sessions.iter().find(|s| s.is_current).unwrap();
        assert_eq!(new_current.name, "project");
    }

    #[test]
    fn test_mock_zellij_tabs() {
        let client = MockZellijClient::new();

        // Create a few tabs
        client.new_tab(Some("code")).unwrap();
        client.new_tab(Some("terminal")).unwrap();

        // List tabs
        let tabs = client.list_tabs().unwrap();
        assert_eq!(tabs.len(), 2);

        // Verify the second tab is active
        assert!(!tabs[0].is_active);
        assert!(tabs[1].is_active);

        // Rename the active tab
        client.rename_tab("console").unwrap();

        // Verify the rename worked
        let updated_tabs = client.list_tabs().unwrap();
        assert_eq!(updated_tabs[1].name, Some("console".to_string()));

        // Close the active tab
        client.close_tab().unwrap();

        // Verify we now have 1 tab and it's active
        let final_tabs = client.list_tabs().unwrap();
        assert_eq!(final_tabs.len(), 1);
        assert!(final_tabs[0].is_active);
    }
}
