use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        /// Include recent Zoxide directories
        #[arg(short, long)]
        all: bool,
    },

    /// Connect to the given session
    #[clap(visible_alias = "cn")]
    Connect {
        /// Session name or part of path
        name: String,
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

    match &cli.command {
        Commands::List { all } => {
            // Include recent Zoxide directories
            if *all {
                let entries = zoxide.list()?;

                println!("Recent directories:");
                for entry in entries {
                    println!("{} {}", entry.score, entry.path.display());
                }
            }

            // List active zellij sessions
            let sessions = zellij.list_sessions()?;

            if sessions.is_empty() {
                println!("No active zellij sessions");
                return Ok(());
            }

            println!("Active zellij sessions:");
            for session in sessions {
                println!(
                    "{}{}",
                    session.name,
                    if session.is_current { " (current)" } else { "" }
                );
            }
        }
        Commands::Connect { name } => {
            // First check if it's an exact session name in zellij
            let sessions = zellij.list_sessions()?;
            let session_match = sessions.iter().find(|s| s.name == *name);

            if let Some(session) = session_match {
                zellij.attach_session(&session.name)?;
                return Ok(());
            }

            // if not a zellij session, check if it is a path
            if let Ok((path, name)) = dir_strategy(name) {
                let session_match = sessions.iter().find(|s| s.name == *name);
                if let Some(session) = session_match {
                    zellij.attach_session(&session.name)?;
                    zoxide.add(path)?;
                    return Ok(());
                } else {
                    env::set_current_dir(&path)?;
                    zellij.new_session(&name)?;
                    zoxide.add(path)?;
                    return Ok(());
                }
            }
            // If not a session name, treat as path search
            let entries = zoxide.query(&[name])?;

            if entries.is_empty() {
                println!("No matching sessions or directories found for '{}'", name);
                return Ok(());
            }

            // Use the highest scored match
            let best_match = &entries[0];
            let path = &best_match.path;
            let session_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("zesh-session");

            if sessions.iter().any(|s| s.name == *session_name) {
                zellij.attach_session(session_name)?;
                return Ok(());
            }

            // Create or attach to session with this path
            println!(
                "Creating new session '{}' at {}",
                session_name,
                path.display()
            );

            // Change to the directory
            env::set_current_dir(path)?;

            // Create new session
            zellij.new_session(session_name)?;

            // Add to zoxide database
            zoxide.add(path)?;
        }

        Commands::Clone {
            repo_url,
            name,
            path,
        } => {
            // Determine the repo name from URL
            let repo_name = extract_repo_name(repo_url)?;
            let session_name = name.as_deref().unwrap_or(repo_name);

            // Determine clone path
            let clone_path = if let Some(p) = path {
                p.join(repo_name)
            } else {
                env::current_dir()?.join(repo_name)
            };

            // Clone the repository
            println!("Cloning {} into {}...", repo_url, clone_path.display());
            let git_output = Command::new("git")
                .arg("clone")
                .arg(repo_url)
                .arg(&clone_path)
                .output()?;

            if !git_output.status.success() {
                let error = String::from_utf8_lossy(&git_output.stderr);
                println!("Git clone failed: {}", error);
                return Ok(());
            }

            println!(
                "Creating new session '{}' at {}",
                session_name,
                clone_path.display()
            );

            // Change to the cloned directory
            env::set_current_dir(&clone_path)?;

            // Create new session
            zellij.new_session(session_name)?;

            // Add to zoxide database
            zoxide.add(&clone_path)?;
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

fn dir_strategy(name: &str) -> anyhow::Result<(PathBuf, String)> {
    let path = PathBuf::from(name).canonicalize()?;
    if !path.exists() {
        return Err(anyhow::anyhow!("Path doesn't exist"));
    }
    if !path.is_dir() {
        return Err(anyhow::anyhow!("Path is not a dir"));
    }

    let final_name = path.clone();
    let final_name = final_name.file_name().and_then(|f| f.to_str());
    match final_name {
        Some(name) => Ok((path, name.to_string())),
        None => Err(anyhow::anyhow!("No file name")),
    }
}
