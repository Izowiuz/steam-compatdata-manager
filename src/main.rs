use clap::Parser;
use std::io::{self, Write};

mod cli;
mod compatdata_manager;

fn confirm(prompt: &str) -> bool {
    print!("{prompt} [y/N]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn main() {
    let args = cli::Cli::parse();

    let mut manager = match args.steam_root {
        Some(path) => match compatdata_manager::CompatdataManager::new(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        None => compatdata_manager::CompatdataManager::discover().unwrap_or_else(|| {
            eprintln!("Could not find Steam installation.");
            std::process::exit(1);
        }),
    };

    println!("Found Steam directory at: {:?}", manager.steam_root());

    if let Err(e) = manager.scan_compatdata_dir() {
        eprintln!("Could not scan compatdata directory: {e}");
        std::process::exit(1);
    }

    print!("\n\n");
    manager.print_entries();

    print!("\n");
    manager.print_summary();

    if args.trash_orphans {
        let orphans = manager.orphans();
        if orphans.is_empty() {
            print!("\n");
            println!("No orphans to remove.");
            return;
        }

        print!("\n");
        println!("Will move {} orphaned prefixes to trash:", orphans.len());
        for o in &orphans {
            println!("  {} {}", o.app_id, o.app_name.as_deref().unwrap_or("-"));
        }

        if !confirm("Proceed?") {
            println!("Cancelled.");
            return;
        }

        for (app_id, result) in manager.trash_orphans() {
            match result {
                Ok(_) => println!("Trashed: {app_id}"),
                Err(e) => eprintln!("Failed {app_id}: {e}"),
            }
        }
    }
}
