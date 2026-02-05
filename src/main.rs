use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {
    old: String,
    new: String,
}

fn main() {
    let cli = Cli::parse();

    println!("Comparing {:?} with {:?}...", cli.new, cli.old);
}
