use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, io};
use walkdir::WalkDir;

pub struct CompatdataManager {
    steamapps_dir: PathBuf,
    compatdata_entries: Vec<CompatdataEntry>,
}

pub struct CompatdataEntry {
    pub app_id: String,
    pub app_name: Option<String>,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub is_orphaned: bool,
}

impl CompatdataManager {
    pub fn new(steamapps_dir: PathBuf) -> Result<Self, String> {
        if !steamapps_dir.is_dir() {
            return Err(format!(
                "Provided steamapps directory {:?} is not a valid directory.",
                steamapps_dir
            ));
        }

        if !steamapps_dir.join("compatdata").is_dir() {
            return Err(format!(
                "Provided steamapps directory {:?} does not contain compatdata directory.",
                steamapps_dir
            ));
        }

        Ok(Self {
            steamapps_dir,
            compatdata_entries: vec![],
        })
    }

    pub fn steamapps_dir(&self) -> &Path {
        &self.steamapps_dir
    }

    pub fn discover() -> Option<Self> {
        let steamapps_dir = find_steamapps_dir()?;
        Some(Self::new(steamapps_dir).ok()?)
    }

    pub fn app_compatdata_path(&self, app_id: u32) -> PathBuf {
        self.steamapps_dir
            .join("compatdata")
            .join(app_id.to_string())
    }

    pub fn scan_compatdata_dir(&mut self) -> Result<(), io::Error> {
        println!("Scanning compatdata directory...");

        let entries: Vec<_> = self
            .steamapps_dir
            .join("compatdata")
            .read_dir()?
            .collect::<Result<_, _>>()?;

        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::with_template("[{bar:40}] {pos}/{len} - {msg}")
                .expect("Could not create progress bar style.")
                .progress_chars("=> "),
        );

        for entry in entries {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(n) => n.to_string_lossy(),
                None => {
                    eprintln!("Could not get file name for path: {:?}", path);
                    continue;
                }
            };

            pb.set_message(format!(
                "scanning app_id: {} at: {}",
                file_name,
                path.display()
            ));

            let size_bytes = Self::calculate_directory_size(&path);
            let app_name =
                Self::get_app_name_from_manifest(&self.steamapps_dir, file_name.to_string());
            let is_orphanded = app_name.is_none();

            pb.inc(1);
        }

        pb.finish_with_message("done.");

        Ok(())
    }

    fn calculate_directory_size(path: &Path) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }

    fn get_app_name_from_manifest(steamapps_dir: &Path, app_id: String) -> Option<String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r#""name"\s+"([^"]+)""#).unwrap());

        let path = steamapps_dir.join(format!("appmanifest_{app_id}.acf"));
        let contents = std::fs::read_to_string(path).ok()?;

        re.captures(&contents)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }
}

impl CompatdataEntry {
    pub fn from_dir_entry() {}
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
                "Could not locate user home directory - pass steamapps directory as an argument."
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
        let compatdata_path = candidate_path.join("compatdata");

        if compatdata_path.is_dir() {
            println!(
                "Using steamapps directory: {:?} from predefined candidates.",
                candidate_path
            );
            return Some(candidate_path);
        }
    }
    None
}
