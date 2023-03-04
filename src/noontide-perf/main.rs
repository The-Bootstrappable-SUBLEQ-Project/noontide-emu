use std::fs::File;

use bincode::deserialize_from;
use clap::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "noontide-perf")]
#[command(author = "NyanCatTW1")]
#[command(about = "Generate and display report from a .perf file", long_about = None)]
struct Cli {
    #[arg(help = "Path to the .perf file")]
    perf_path: String,
}

fn main() {
    let cli = Cli::parse();
    let perf_path = cli.perf_path;

    let recorded_eips: HashMap<u64, u64> =
        deserialize_from(File::open(perf_path).unwrap()).unwrap();
    for (key, value) in &recorded_eips {
        println!("{}: {}", key, value);
    }
}
