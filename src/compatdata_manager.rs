use std::env;
use std::path::{Path, PathBuf};

pub struct CompatdataManager {
    steamapps_dir: PathBuf,
}

impl CompatdataManager {
    pub fn new(steamapps_dir: PathBuf) -> Self {
        Self { steamapps_dir }
    }

    pub fn discover() -> Option<Self> {
        let steamapps_dir = find_steamapps_dir()?;
        Some(Self::new(steamapps_dir))
    }

    pub fn app_compatdata_path(&self, app_id: u32) -> PathBuf {
        self.steamapps_dir
            .join("compatdata")
            .join(app_id.to_string())
    }

    pub fn steamapps_dir(&self) -> &Path {
        &self.steamapps_dir
    }
}

fn find_steamapps_dir() -> Option<PathBuf> {
    const CANDIDATES: [&str; 4] = [
        ".steam/steam/steamapps",
        "steamapps",
        "Steamapps",
        ".var/app/com.valvesoftware.Steam/data/Steam/steamapps",
    ];

    let home_dir = match env::home_dir() {
        Some(dir) => dir,
        None => {
            eprintln!(
                "Could not locate user home directory - pass steamapps directory as argument."
            );
            return None;
        }
    };

    println!("Will search for steamapps directory in candidates:");
    for c in CANDIDATES {
        println!("  - {}", home_dir.join(c).display());
    }

    for candidate in CANDIDATES {
        let candidate_path = home_dir.join(candidate);
        if candidate_path.exists() {
            println!(
                "Using steamapps directory: {:?} from predefined candidates.",
                candidate_path
            );
            return Some(candidate_path);
        }
    }
    None
}
