use std::sync::Arc;

use bitcoin::Network;
use clap::{Parser, ValueEnum};

use crate::backend::{BitcoinAddressType, BitcoinBackend, SolanaBackend, VanityBackend};
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

    /// Select blockchain backend
    #[arg(long = "chain", value_enum, default_value_t = ChainChoice::Solana)]
    chain: ChainChoice,

    /// Bitcoin address type (legacy=1..., segwit=bc1q...)
    #[arg(long = "type", value_enum, default_value_t = AddressTypeChoice::Legacy)]
    address_type: AddressTypeChoice,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ChainChoice {
    Solana,
    Bitcoin,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum AddressTypeChoice {
    Legacy,
    Segwit,
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
    let prefixes = apply_chain_prefixes(prefixes, args.chain, args.address_type);

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

    let backend: Arc<dyn VanityBackend> = match args.chain {
        ChainChoice::Solana => Arc::new(SolanaBackend),
        ChainChoice::Bitcoin => Arc::new(BitcoinBackend::new(
            Network::Bitcoin,
            BitcoinAddressType::from(args.address_type),
        )),
    };

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

fn apply_chain_prefixes(
    prefixes: Vec<String>,
    chain: ChainChoice,
    address_type: AddressTypeChoice,
) -> Vec<String> {
    if !matches!(chain, ChainChoice::Bitcoin) {
        return prefixes;
    }

    prefixes
        .into_iter()
        .map(|raw| match address_type {
            AddressTypeChoice::Legacy => {
                if raw.starts_with('1') {
                    raw
                } else {
                    format!("1{raw}")
                }
            }
            AddressTypeChoice::Segwit => {
                let lowered = raw.to_lowercase();
                if lowered.starts_with("bc1q") {
                    lowered
                } else {
                    format!("bc1q{lowered}")
                }
            }
        })
        .collect()
}

impl From<AddressTypeChoice> for BitcoinAddressType {
    fn from(choice: AddressTypeChoice) -> Self {
        match choice {
            AddressTypeChoice::Legacy => BitcoinAddressType::Legacy,
            AddressTypeChoice::Segwit => BitcoinAddressType::Segwit,
        }
    }
}
