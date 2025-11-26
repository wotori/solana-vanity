use std::sync::Arc;

use clap::Parser;

use crate::backend::{SolanaBackend, VanityBackend};
use crate::matcher::PrefixRule;

#[derive(Parser)]
#[command(author, version, about = "Solana vanity address finder")]
struct Args {
    /// One or more desired prefixes for the base58-encoded public key
    #[arg(long = "prefix", value_name = "PREFIX")]
    prefixes: Vec<String>,

    /// Stop after this many matches (0 = run forever)
    #[arg(long = "max-matches", default_value_t = 1)]
    max_matches: u64,

    /// Treat prefix matching as case-insensitive (ASCII)
    #[arg(long = "ignore-case", default_value_t = false)]
    ignore_case: bool,
}

#[derive(Clone)]
pub struct Config {
    pub prefix_rules: Arc<Vec<PrefixRule>>,
    pub max_matches: u64,
    pub ignore_case: bool,
    pub threads: usize,
    pub backend: Arc<dyn VanityBackend>,
}

pub fn parse_config() -> Config {
    let args = Args::parse();
    let prefixes = expand_prefixes(&args.prefixes);

    let prefix_rules = Arc::new(
        prefixes
            .into_iter()
            .map(|raw| PrefixRule {
                bytes: raw.as_bytes().to_vec(),
                raw,
            })
            .collect::<Vec<_>>(),
    );

    let threads = num_cpus::get();

    let backend: Arc<dyn VanityBackend> = Arc::new(SolanaBackend);

    Config {
        prefix_rules,
        max_matches: args.max_matches,
        ignore_case: args.ignore_case,
        threads,
        backend,
    }
}

fn expand_prefixes(raw_list: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    if raw_list.is_empty() {
        expanded.push("xyber".to_string());
        return expanded;
    }

    for item in raw_list {
        for part in item.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                expanded.push(trimmed.to_string());
            }
        }
    }

    if expanded.is_empty() {
        expanded.push("xyber".to_string());
    }

    expanded
}
