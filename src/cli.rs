use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(
    version,
    about = "Simple tool to manage (read - clean) steam compatdata"
)]
pub struct Cli {
    /// Path to the steam root directory
    ///
    /// If not provided, the tool will try some standard locations:
    /// "[HOME]/.steam/steam",
    /// "[HOME]/.local/share/Steam",
    /// "[HOME]/.var/app/com.valvesoftware.Steam/data/Steam
    #[arg(long)]
    pub steam_root: Option<PathBuf>,

    /// Move orphaned prefixes to trash
    #[arg(long)]
    pub trash_orphans: bool,
}
