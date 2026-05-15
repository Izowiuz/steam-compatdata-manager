use clap::Parser;

mod cli;
mod compatdata_manager;

fn main() {
    let args = cli::Cli::parse();

    let manager = match args.steamapps_dir {
        Some(path) => compatdata_manager::CompatdataManager::new(path),
        None => match compatdata_manager::CompatdataManager::discover() {
            Some(manager) => manager,
            None => {
                eprintln!("Could not find steamapps directory.");
                return;
            }
        },
    };

    println!(
        "Found steamapps directory at: {:?}",
        manager.steamapps_dir()
    );
}
