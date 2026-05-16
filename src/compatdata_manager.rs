use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;
use tabled::builder::Builder;
use tabled::settings::Style;
use walkdir::WalkDir;

pub struct CompatdataManager {
    steam_root: PathBuf,
    library_paths: Vec<PathBuf>,
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
    pub fn new(steam_root: PathBuf) -> Result<Self, String> {
        if !steam_root.is_dir() {
            return Err(format!(
                "Provided Steam root {:?} is not a valid directory.",
                steam_root
            ));
        }

        let vdf = steam_root.join("config/libraryfolders.vdf");
        if !vdf.is_file() {
            return Err(format!("No libraryfolders.vdf found at {:?}", vdf));
        }

        let library_paths = discover_library_paths(&steam_root);

        if library_paths.is_empty() {
            return Err(format!("No Steam libraries found in {:?}", vdf));
        }

        Ok(Self {
            steam_root,
            library_paths,
            compatdata_entries: vec![],
        })
    }

    pub fn steam_root(&self) -> &Path {
        &self.steam_root
    }

    pub fn discover() -> Option<Self> {
        let steam_root = find_steam_root()?;
        Some(Self::new(steam_root).ok()?)
    }

    pub fn scan_compatdata_dir(&mut self) -> Result<(), io::Error> {
        println!("Scanning compatdata...");

        let entries: Vec<_> = self
            .steam_root
            .join("steamapps")
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

            let mut app_name = None;
            let mut is_orphaned = false;
            let mut is_unknown = false;

            if let Some(name) = self.get_app_name_from_manifest(&app_id) {
                app_name = Some(name);
            } else if let Some(name) = Self::fetch_game_name_from_store(&app_id) {
                app_name = Some(name);
                is_orphaned = true;
            } else {
                is_unknown = true;
            }

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

    fn get_app_name_from_manifest(&self, app_id: &str) -> Option<String> {
        static MANIFEST_NAME_RE: OnceLock<Regex> = OnceLock::new();
        let re = MANIFEST_NAME_RE.get_or_init(|| Regex::new(r#""name"\s+"([^"]+)""#).unwrap());

        for lib in &self.library_paths {
            let path = lib.join(format!("appmanifest_{app_id}.acf"));
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Some(cap) = re.captures(&contents) {
                    return cap.get(1).map(|m| m.as_str().to_string());
                }
            }
        }
        None
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
                e.app_name.clone().unwrap_or_else(|| "-".into()),
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

    pub fn orphans(&self) -> Vec<&CompatdataEntry> {
        self.compatdata_entries
            .iter()
            .filter(|e| e.is_orphaned)
            .collect()
    }

    pub fn trash_orphans(&self) -> Vec<(String, Result<(), trash::Error>)> {
        self.orphans()
            .iter()
            .map(|e| (e.app_id.clone(), trash::delete(&e.path)))
            .collect()
    }
}

fn find_steam_root() -> Option<PathBuf> {
    const STEAM_ROOT_CANDIDATES: [&str; 3] = [
        ".steam/steam",
        ".local/share/Steam",
        ".var/app/com.valvesoftware.Steam/data/Steam",
    ];

    let home = std::env::home_dir()?;
    STEAM_ROOT_CANDIDATES
        .iter()
        .map(|s| home.join(s))
        .find(|p| p.join("config/libraryfolders.vdf").exists())
}

fn discover_library_paths(steam_root: &Path) -> Vec<PathBuf> {
    static LIB_PATH_RE: OnceLock<Regex> = OnceLock::new();
    let re = LIB_PATH_RE.get_or_init(|| Regex::new(r#""path"\s+"([^"]+)""#).unwrap());

    let vdf = steam_root.join("config/libraryfolders.vdf");
    let content = std::fs::read_to_string(&vdf).unwrap_or_default();

    let paths: Vec<PathBuf> = re
        .captures_iter(&content)
        .map(|cap| PathBuf::from(cap.get(1).unwrap().as_str()).join("steamapps"))
        .collect();

    println!(
        "Found {} Steam libraries in {}:",
        paths.len(),
        vdf.display()
    );

    for p in &paths {
        println!("  {}", p.display());
    }

    paths
}
