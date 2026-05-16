use clap::Parser;

mod cli;
mod compatdata_manager;

fn main() {
    let args = cli::Cli::parse();

    let mut manager = match args.steamapps_dir {
        Some(path) => match compatdata_manager::CompatdataManager::new(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        None => compatdata_manager::CompatdataManager::discover().unwrap_or_else(|| {
            eprintln!("Could not find steamapps directory.");
            std::process::exit(1);
        }),
    };

    println!(
        "Found steamapps directory at: {:?}",
        manager.steamapps_dir()
    );

    if let Err(e) = manager.scan_compatdata_dir() {
        eprintln!("Could not scan steamdata directory: {e}");
        std::process::exit(1);
    }

    print!("\n\n");
    manager.print_entries();
    print!("\n");
    manager.print_summary();
}
