use clap::Parser;
use std::fs;
use std::io;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    old: String,
    new: String,
}

fn run(cli: Cli) -> io::Result<()> {
    let old_content = fs::read_to_string(&cli.old)?;
    let new_content = fs::read_to_string(&cli.new)?;

    println!("Old ({} bytes):\n{old_content}", old_content.len());
    println!("New ({} bytes):\n{new_content}", new_content.len());

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
