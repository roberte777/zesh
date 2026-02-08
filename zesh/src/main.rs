use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use zellij_rs::options::ZellijOptions;
use zesh::clone::CloneService;
use zesh::connection::ConnectService;
use zesh::fs::RealFs;
use zesh_git::RealGit;

use zellij_rs::{ZellijClient, ZellijOperations};
use zox_rs::{ZoxideClient, ZoxideOperations};

/// Zesh - A zellij session manager with zoxide integration
#[derive(Parser)]
#[clap(version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List sessions
    #[clap(visible_alias = "l")]
    List {
        /// Show only zellij sessions
        #[clap(short = 'Z', long)]
        zesh: bool,

        /// Show only zoxide results
        #[clap(short, long)]
        zoxide: bool,

        /// Output as JSON
        #[clap(short, long)]
        json: bool,

        /// Hide the currently attached zellij session
        #[clap(short = 'H', long)]
        hide_attached: bool,

        /// Hide duplicate entries (by name)
        #[clap(short = 'd', long)]
        hide_duplicates: bool,
    },

    /// Connect to the given session. Zellij arguments are only passed if
    /// creating a new session
    #[clap(visible_alias = "cn")]
    Connect {
        /// Session name or part of path
        name: String,
        #[clap(flatten)]
        zellij_options: ZellijOptions,
    },

    /// Clone a git repo and connect to it as a session
    #[clap(visible_alias = "cl")]
    Clone {
        /// Repository URL to clone
        repo_url: String,

        /// Optional custom session name (defaults to repo name)
        #[clap(long)]
        name: Option<String>,

        /// Optional path to clone into (defaults to current directory)
        #[clap(long)]
        path: Option<PathBuf>,
        /// Zellij options
        #[clap(flatten)]
        zellij_options: ZellijOptions,
    },

    /// Show the root directory from the active session
    #[clap(visible_alias = "r")]
    Root,

    /// Preview a session or directory
    #[clap(visible_alias = "p")]
    Preview {
        /// Session name or directory path
        target: String,
    },
}

/// A list entry for output (used for both display and JSON serialization)
#[derive(Debug, Serialize)]
struct ListEntry {
    /// The source of this entry: "zellij" or "zoxide"
    src: String,
    /// Display name (session name or shortened path)
    name: String,
    /// Absolute path (only for zoxide entries)
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// Zoxide score (only for zoxide entries)
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
}

/// Shorten a path by replacing the home directory prefix with ~
fn shorten_home(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(suffix) = path.strip_prefix(&home) {
            return format!("~/{}", suffix.display());
        }
    }
    path.display().to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let zellij = ZellijClient::new();
    let zoxide = ZoxideClient::new();
    let fs = RealFs::new();
    let git = RealGit;

    let connect_service = ConnectService::new(zellij, zoxide, fs, git);

    match &cli.command {
        Commands::List {
            zesh,
            zoxide: zoxide_only,
            json,
            hide_attached,
            hide_duplicates,
        } => {
            // If no source flags, show all sources. If any source flag is set,
            // show only the requested sources.
            let show_all = !zesh && !zoxide_only;
            let show_zellij = show_all || *zesh;
            let show_zoxide = show_all || *zoxide_only;

            let mut entries: Vec<ListEntry> = Vec::new();

            // Zellij sessions first (matching sesh's default order: sessions before zoxide)
            if show_zellij {
                let sessions = zellij.list_sessions()?;
                for session in &sessions {
                    if *hide_attached && session.is_current {
                        continue;
                    }
                    entries.push(ListEntry {
                        src: "zellij".to_string(),
                        name: session.name.clone(),
                        path: None,
                        score: None,
                    });
                }
            }

            // Zoxide entries
            if show_zoxide {
                let zoxide_entries = zoxide.list()?;
                for entry in &zoxide_entries {
                    entries.push(ListEntry {
                        src: "zoxide".to_string(),
                        name: shorten_home(&entry.path),
                        path: Some(entry.path.display().to_string()),
                        score: Some(entry.score),
                    });
                }
            }

            // Deduplication: remove entries with duplicate names
            if *hide_duplicates {
                let mut seen = HashSet::new();
                entries.retain(|e| seen.insert(e.name.clone()));
            }

            // Output
            if *json {
                let json_str = serde_json::to_string(&entries)?;
                println!("{}", json_str);
            } else {
                for entry in &entries {
                    println!("{}", entry.name);
                }
            }
        }
        Commands::Connect {
            name,
            zellij_options,
        } => {
            // Use our new connect service
            if let Err(e) = connect_service.connect(name, zellij_options) {
                eprintln!("Error connecting to '{}': {}", name, e);
                return Err(e.into());
            }
        }

        Commands::Clone {
            repo_url,
            name,
            path,
            zellij_options,
        } => {
            let clone_service = CloneService::new(zellij, zoxide, fs, git);
            if let Err(e) = clone_service.clone_repo(
                repo_url,
                name.as_deref(),
                path.as_ref(),
                zellij_options,
            ) {
                eprintln!("Clone failed: {}", e);
                return Err(e.into());
            }
        }

        Commands::Root => {
            // Get current session
            let sessions = zellij.list_sessions()?;
            let current = sessions.iter().find(|s| s.is_current);

            if let Some(_session) = current {
                // Assume session name is the directory name
                // This is a simplification - you might want to store session roots somewhere
                println!("{}", env::current_dir()?.display());
            } else {
                println!("No active zellij session");
            }
        }

        Commands::Preview { target } => {
            // First check if it's a session
            let sessions = zellij.list_sessions()?;
            let session_match = sessions.iter().find(|s| s.name == *target);

            if let Some(session) = session_match {
                println!("Session: {}", session.name);
                // In a real implementation, you'd show more details about the session
                return Ok(());
            }

            // If not a session, check if it's a directory
            let path = PathBuf::from(target);
            if path.is_dir() {
                println!("Directory: {}", path.display());
                preview_directory(&path)?;
                return Ok(());
            }

            // If not a directory, try zoxide query
            let entries = zoxide.query(&[target])?;

            if entries.is_empty() {
                println!("No matching sessions or directories found for '{}'", target);
                return Ok(());
            }

            // Use the highest scored match
            let best_match = &entries[0];
            println!("Directory (via zoxide): {}", best_match.path.display());
            preview_directory(&best_match.path)?;
        }
    }

    Ok(())
}

/// Preview directory contents
fn preview_directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Print a basic directory listing
    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_type = if metadata.is_dir() {
            "dir"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        println!("{:<6} {}", file_type, entry.file_name().to_string_lossy());
    }

    Ok(())
}
