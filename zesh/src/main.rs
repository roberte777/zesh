use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use zellij_rs::options::ZellijOptions;
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
    List,

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

        /// Extra arguments passed to git clone (after --)
        #[clap(last = true)]
        git_args: Vec<String>,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let zellij = ZellijClient::new();
    let zoxide = ZoxideClient::new();
    let fs = RealFs::new();
    let git = RealGit;

    let connect_service = ConnectService::new(zellij, zoxide, fs, git);

    match &cli.command {
        Commands::List => {
            // List all directories from zoxide
            let entries = zoxide.list()?;
            for entry in entries {
                println!("{}", entry.path.display());
            }
            // List active zellij sessions
            let sessions = zellij.list_sessions()?;

            if sessions.is_empty() {
                return Ok(());
            }

            for session in sessions {
                println!("{}", session.name,);
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
            git_args,
        } => {
            let repo_name = extract_repo_name(repo_url)?;
            let session_name = name.as_deref().unwrap_or(repo_name);

            // cmd_dir = --path value or empty (let clone_and_connect use cwd)
            let cmd_dir = match path {
                Some(p) => p.display().to_string(),
                None => String::new(),
            };

            println!("Cloning {}...", repo_url);

            if let Err(e) = connect_service.clone_and_connect(
                repo_url,
                session_name,
                &cmd_dir,
                repo_name,
                git_args,
                zellij_options,
            ) {
                eprintln!("Clone failed: {}", e);
                return Err(e.into());
            }

            println!("Created session '{}'", session_name);
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

/// Extract repository name from URL
fn extract_repo_name(url: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let url = url.trim_end_matches(".git");

    url.rsplit('/')
        .next()
        .ok_or_else(|| "Could not parse repository name from URL".into())
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
