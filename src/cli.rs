use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(version, about = "Simple tool to manage steam compatdata")]
pub struct Cli {
    /// Path to the steamapps directory
    ///
    /// If not provided, the tool will try some standard locations:
    /// "[HOME]/.steam/steam/steamapps",
    /// "[HOME]/steamapps",
    /// "[HOME]/Steamapps",
    /// "[HOME]/.var/app/com.valvesoftware.Steam/data/Steam/steamapps
    #[arg(long)]
    pub steamapps_dir: Option<PathBuf>,
}
