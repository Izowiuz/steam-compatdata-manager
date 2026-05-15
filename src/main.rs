mod compatdata_manager;

fn main() {
    let compatdata_manager = match compatdata_manager::CompatdataManager::discover() {
        Some(manager) => manager,
        None => {
            eprintln!("Could not find steamapps directory.");
            return;
        }
    };

    println!(
        "Found steamapps directory at: {:?}",
        compatdata_manager.steamapps_dir
    );
}
