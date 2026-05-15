use std::env;
use std::path::PathBuf;

pub struct CompatdataManager {
    pub steamapps_dir: PathBuf,
}

impl CompatdataManager {
    pub fn new(steamapps_dir: PathBuf) -> Self {
        Self { steamapps_dir }
    }

    pub fn discover() -> Option<Self> {
        let steamapps_dir = find_steamapps_dir()?;
        Some(Self::new(steamapps_dir))
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

    for candidate in CANDIDATES.iter() {
        // TODO: better logging
        println!("Will search in: {:?}", CANDIDATES);

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
