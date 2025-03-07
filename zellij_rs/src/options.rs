use clap::Args;

/// Options for zellij commands
#[derive(Debug, Clone, Default, Args)]
pub struct ZellijOptions {
    /// Name of a predefined layout or path to a layout file
    #[arg(short, long)]
    pub new_session_with_layout: Option<String>,

    /// Path to config file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to config directory
    #[arg(short = 'C', long)]
    pub config_dir: Option<String>,

    /// Path to data directory for plugins
    #[arg(short, long)]
    pub data_dir: Option<String>,

    /// Maximum panes on screen
    #[arg(short, long)]
    pub max_panes: Option<u32>,

    /// Enable debug output
    #[arg(short, long)]
    pub debug: bool,
}
