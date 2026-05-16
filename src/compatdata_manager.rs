use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;
use std::{env, io};
use tabled::builder::Builder;
use tabled::settings::Style;
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
    pub is_unknown: bool,
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

    pub fn scan_compatdata_dir(&mut self) -> Result<(), io::Error> {
        println!("Scanning compatdata directory...");

        let entries: Vec<_> = self
            .steamapps_dir
            .join("compatdata")
            .read_dir()?
            .collect::<Result<_, _>>()?;

        let progress_bar = ProgressBar::new(entries.len() as u64);
        progress_bar.set_style(
            ProgressStyle::with_template("[{bar:40}] {pos}/{len} - {msg}")
                .expect("Could not create progress bar style.")
                .progress_chars("=> "),
        );

        let mut compatdata_entries: Vec<CompatdataEntry> = vec![];

        for entry in entries {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(n) => n.to_string_lossy(),
                None => {
                    eprintln!("Could not get file name for path: {:?}", path);
                    continue;
                }
            };

            progress_bar.set_message(format!(
                "scanning app_id: {} at: {}",
                file_name,
                path.display()
            ));

            let app_id = file_name.to_string();
            let size_bytes = Self::calculate_directory_size(&path);
            let mut is_unknown = false;
            let app_name = match Self::get_app_name_from_manifest(&self.steamapps_dir, &app_id) {
                Some(name) => Some(name),
                None => match Self::fetch_game_name_from_store(&app_id) {
                    Some(name) => Some(name),
                    None => {
                        is_unknown = true;
                        None
                    }
                },
            };

            let is_orphaned = app_name.is_none();

            compatdata_entries.push(CompatdataEntry {
                app_id,
                app_name,
                path,
                size_bytes,
                is_orphaned: if is_unknown { false } else { is_orphaned },
                is_unknown,
            });

            progress_bar.inc(1);
        }

        compatdata_entries.sort_by(|a, b| {
            b.is_orphaned
                .cmp(&a.is_orphaned)
                .then(b.size_bytes.cmp(&a.size_bytes))
        });

        self.compatdata_entries = compatdata_entries;

        progress_bar.finish_with_message("done.");

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

    fn get_app_name_from_manifest(steamapps_dir: &Path, app_id: &str) -> Option<String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r#""name"\s+"([^"]+)""#).unwrap());

        let path = steamapps_dir.join(format!("appmanifest_{app_id}.acf"));
        let contents = std::fs::read_to_string(path).ok()?;

        re.captures(&contents)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        match bytes {
            b if b >= GB => format!("{:.2} GiB", b as f64 / GB as f64),
            b if b >= MB => format!("{:.1} MiB", b as f64 / MB as f64),
            b if b >= KB => format!("{:.1} KiB", b as f64 / KB as f64),
            b => format!("{b} B"),
        }
    }

    fn fetch_game_name_from_store(app_id: &str) -> Option<String> {
        sleep(Duration::from_millis(1500));

        let url =
            format!("https://store.steampowered.com/api/appdetails?appids={app_id}&filters=basic");
        let mut response = ureq::get(&url).call().ok()?;
        let body: serde_json::Value = response.body_mut().read_json().ok()?;

        let entry = body.get(app_id.to_string())?;
        if !entry.get("success")?.as_bool()? {
            return None;
        }
        entry.get("data")?.get("name")?.as_str().map(str::to_string)
    }

    pub fn print_entries(&self) {
        let mut builder = Builder::default();
        builder.push_record(["AppID", "Name", "Path", "Size", "Orphan", "Unknown"]);

        for e in &self.compatdata_entries {
            builder.push_record([
                e.app_id.to_string(),
                e.app_name.clone().unwrap_or_else(|| "—".into()),
                e.path.display().to_string(),
                Self::format_size(e.size_bytes),
                if e.is_orphaned { "yes" } else { "no" }.into(),
                if e.is_unknown { "yes" } else { "no" }.into(),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::psql());

        println!("{table}");
    }

    pub fn print_summary(&self) {
        let total = self.compatdata_entries.len();
        let orphans = self
            .compatdata_entries
            .iter()
            .filter(|e| e.is_orphaned)
            .count();
        let total_size: u64 = self.compatdata_entries.iter().map(|e| e.size_bytes).sum();
        let orphan_size: u64 = self
            .compatdata_entries
            .iter()
            .filter(|e| e.is_orphaned)
            .map(|e| e.size_bytes)
            .sum();

        println!(
            "Total: {} prefixes ({} orphans), {} on disk ({} reclaimable)",
            total,
            orphans,
            Self::format_size(total_size),
            Self::format_size(orphan_size),
        );
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
