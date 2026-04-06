use std::process::{Command, exit};
use std::thread::sleep;
use std::time::Duration;

/// Crates in dependency order — each crate's dependencies have already been
/// published by the time it is reached.
const CRATES: &[&str] = &["faputa", "faputa_meta", "faputa_generator", "faputa_derive"];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("publish") => publish(false),
        Some("publish-dry") => publish(true),
        _ => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  publish      Publish crates to crates.io (in dependency order)");
            eprintln!("  publish-dry  Dry-run publish (no actual upload)");
            exit(1);
        }
    }
}

fn publish(dry_run: bool) {
    let mode = if dry_run { " (dry run)" } else { "" };
    println!("Publishing faputa crates{mode}...\n");

    for (i, crate_name) in CRATES.iter().enumerate() {
        println!("[{}/{}] Publishing {crate_name}...", i + 1, CRATES.len());

        let mut cmd = Command::new("cargo");
        cmd.arg("publish").arg("-p").arg(crate_name);

        if dry_run {
            cmd.arg("--dry-run").arg("--allow-dirty");
        }

        let status = cmd.status().expect("failed to execute cargo publish");

        if !status.success() {
            eprintln!("Failed to publish {crate_name}");
            exit(1);
        }

        println!("{crate_name} published successfully\n");

        // Wait for crates.io index sync between dependent crates
        if !dry_run && i < CRATES.len() - 1 {
            println!("Waiting 15s for crates.io index sync...");
            sleep(Duration::from_secs(15));
        }
    }

    println!("All crates published!");
}
