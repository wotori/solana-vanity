mod backend;
mod config;
mod matcher;
mod miner;
mod reporter;

use config::parse_config;
use miner::mine_one_round;
use reporter::print_and_save_result;

fn main() {
    let config = parse_config();

    println!("Using {} threads", config.threads);
    println!(
        "Searching for prefixes: {}",
        config
            .prefix_rules
            .iter()
            .map(|p| p.raw.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    match config.max_matches {
        0 => println!("Will keep mining indefinitely."),
        1 => println!("Will stop after finding 1 match."),
        n => println!("Will stop after finding {n} matches."),
    }

    let mut matches_found = 0u64;
    while config.max_matches == 0 || matches_found < config.max_matches {
        let run_index = matches_found + 1;
        println!("--- Run #{run_index} ---");

        let result = mine_one_round(&config);
        print_and_save_result(&result);

        matches_found += 1;
    }

    println!("Finished after {matches_found} matches.");
}
