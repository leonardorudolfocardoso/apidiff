mod change;
mod diff;
mod loader;

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    old: PathBuf,
    new: PathBuf,
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let old_spec = loader::load_file(&cli.old)?;
    let new_spec = loader::load_file(&cli.new)?;

    let diff = diff::diff_specs(&old_spec, &new_spec);

    if diff.is_empty() {
        println!("No changes detected.");
        return Ok(());
    }

    let breaking = diff.breaking();
    let non_breaking = diff.non_breaking();

    if !breaking.is_empty() {
        println!("Breaking changes ({}):", breaking.len());
        for c in &breaking {
            println!("  {c}");
        }
    }

    if !non_breaking.is_empty() {
        if !breaking.is_empty() {
            println!();
        }
        println!("Non-breaking changes ({}):", non_breaking.len());
        for c in &non_breaking {
            println!("  {c}");
        }
    }

    if diff.has_breaking() {
        std::process::exit(1);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(2);
    }
}
