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

    // Load the debug data from hex*, if any
    let mut debug_data: Option<pdb::DebugData> = None;
    for ext in ["hex0", "hex1", "hex2"] {
        let mut hex_path_str = base_path.clone();
        hex_path_str.push('.');
        hex_path_str.push_str(ext);
        let hex_path = std::path::Path::new(&hex_path_str);
        if !hex_path.exists() {
            continue;
        }

        debug_data = Some(pdb::parse_hex_file(
            &std::fs::read_to_string(hex_path).unwrap(),
        ));
    }

    let lines = debug_data.unwrap().offsets;
    let mut i = 0;
    let mut total_hits = 0;
    let mut hits_per_line: Vec<(u64, String)> = Vec::new();
    for record in recorded_eips {
        total_hits += record.1;

        if i == lines.len() {
            continue;
        }

        while i + 1 != lines.len() && lines[i + 1].0 <= record.0 {
            hits_per_line.push((0, lines[i].1.clone()));
            i += 1;
        }

        hits_per_line.push((record.1, lines[i].1.clone()));
        i += 1;
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
        } else {
            println!("{: >8} | {}", hits_str, line);
        }
    }
}
