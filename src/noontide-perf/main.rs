use std::fs::File;

use bincode::deserialize_from;
use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;

use noontide_emu::pdb;

#[derive(Parser)]
#[command(name = "noontide-perf")]
#[command(author = "NyanCatTW1")]
#[command(about = "Generate and display report from a .perf file", long_about = None)]
struct Cli {
    #[arg(help = "Path to the .perf file")]
    perf_path: String,

    #[arg(help = "Base path to hex*, lsq, and msq files, without the file extension")]
    base_path: String,
}

fn main() {
    let cli = Cli::parse();
    let perf_path = cli.perf_path;
    let base_path = cli.base_path;

    let recorded_eips_hashmap: HashMap<u64, u64> =
        deserialize_from(File::open(perf_path).unwrap()).unwrap();
    let mut recorded_eips: Vec<(u64, u64)> = recorded_eips_hashmap.into_iter().collect();
    recorded_eips.sort();

    let debug_data = pdb::find_debug_data(&base_path);
    let lines = debug_data.unwrap().offsets;
    let mut i = 0;
    let mut cur_hits = 0;
    let mut total_hits = 0;
    let mut hits_per_line: Vec<(u64, String)> = Vec::new();
    for record in recorded_eips {
        total_hits += record.1;

        if i == lines.len() {
            continue;
        }

        while i + 1 != lines.len() && lines[i + 1].0 <= record.0 {
            hits_per_line.push((cur_hits, lines[i].1.clone()));
            cur_hits = 0;
            i += 1;
        }

        cur_hits += record.1;
    }

    for (hits, line) in hits_per_line {
        let percentage = (hits as f64 * 100.0) / total_hits as f64;
        let hits_str = if hits == 0 {
            "".to_owned()
        } else {
            format!("{:.2}", percentage)
        };

        if percentage >= 1.0 {
            println!("{: >8} | {}", hits_str.red(), line.red());
        } else if percentage >= 0.1 {
            println!("{: >8} | {}", hits_str.green(), line.green());
        } else {
            println!("{: >8} | {}", hits_str, line);
        }
    }
}
