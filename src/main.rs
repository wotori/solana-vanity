use chrono::{SecondsFormat, Utc};
use clap::Parser;
use ed25519_dalek::Keypair;
use rand::{rngs::OsRng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rayon::scope;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(author, version, about = "Solana vanity address finder")]
struct Args {
    /// One or more desired prefixes for the base58-encoded public key
    #[arg(long = "prefix", value_name = "PREFIX")]
    prefixes: Vec<String>,

    #[arg(long = "max-matches", default_value_t = 1)]
    max_matches: u64,

    /// Treat prefix matching as case-insensitive (ASCII)
    #[arg(long = "ignore-case", default_value_t = false)]
    ignore_case: bool,
}

fn main() {
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
    let max_matches = args.max_matches;
    let ignore_case = args.ignore_case;
    let threads = num_cpus::get();
    println!("Using {} threads", threads);
    println!(
        "Searching for prefixes: {}",
        prefix_rules
            .iter()
            .map(|p| p.raw.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    match max_matches {
        0 => println!("Will keep mining indefinitely."),
        1 => println!("Will stop after finding 1 match."),
        n => println!("Will stop after finding {} matches.", n),
    }

    let mut matches_found = 0u64;
    loop {
        if max_matches != 0 && matches_found >= max_matches {
            break;
        }

        let run_index = matches_found + 1;
        println!("--- Run #{} ---", run_index);

        let found = Arc::new(AtomicBool::new(false));
        let attempts = Arc::new(AtomicU64::new(0));
        let start = Instant::now();
        let start_for_logger = start;
        let logger_attempts = attempts.clone();
        let logger_found = found.clone();
        let log_handle = thread::spawn(move || {
            let interval = Duration::from_secs(5);
            loop {
                thread::sleep(interval);
                let total = logger_attempts.load(Ordering::Relaxed);
                let elapsed = start_for_logger.elapsed().as_secs_f64().max(1.0);
                let rate = (total as f64 / elapsed).round() as u64;
                let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
                let log_line = format!(
                    "[{}] Total attempts: {}  |  Rate: {} attempts/s",
                    timestamp,
                    format_with_commas(total),
                    format_with_commas(rate)
                );
                println!("{}", log_line);

                if logger_found.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        let prefix_rules_for_threads = prefix_rules.clone();

        scope(|s| {
            for _ in 0..threads {
                let found = found.clone();
                let attempts = attempts.clone();
                let start = start;
                let rules = prefix_rules_for_threads.clone();

                s.spawn(move |_| {
                    let mut rng = ChaCha20Rng::from_rng(OsRng).expect("seed rng");
                    let mut buffer = [0u8; 64];
                    let mut local_attempts = 0u64;

                    while !found.load(Ordering::Relaxed) {
                        let kp = Keypair::generate(&mut rng);

                        local_attempts += 1;
                        if local_attempts >= 100_000 {
                            attempts.fetch_add(local_attempts, Ordering::Relaxed);
                            local_attempts = 0;
                        }

                        let len = bs58::encode(kp.public.to_bytes())
                            .onto(&mut buffer[..])
                            .unwrap();

                        let candidate = &buffer[..len];
                        if let Some(rule) =
                            rules
                                .iter()
                                .find(|rule| candidate_matches(candidate, &rule.bytes, ignore_case))
                        {
                            if !found.swap(true, Ordering::Relaxed) {
                                attempts.fetch_add(local_attempts, Ordering::Relaxed);

                                println!("Found match!");
                                let pub58_str = std::str::from_utf8(candidate).unwrap();
                                println!("Public key: {}", pub58_str);

                                let secret_full = kp.to_bytes();
                                let secret_hex = hex::encode(secret_full);
                                println!("Secret key (hex): {}", secret_hex);

                                let elapsed = start.elapsed().as_secs_f64();
                                let total_attempts = attempts.load(Ordering::Relaxed);

                                println!(
                                    "Attempts: {} ({:.0} / sec)",
                                    total_attempts,
                                    total_attempts as f64 / elapsed
                                );

                                let rate = (total_attempts as f64 / elapsed).round();
                                let secret_bytes = format!("{:?}", secret_full);
                                let public_bytes = format!("{:?}", kp.public.to_bytes());
                                let secret_json = serde_json::to_string(&secret_full.to_vec())
                                    .unwrap_or_else(|_| "[]".to_string());

                                let summary = format!(
                                    "[{}] Prefix: {}\nPublic key: {}\nPublic key (bytes): {}\nSecret key (hex): {}\nSecret key (bytes): {}\nSecret key (json array): {}\nAttempts: {}\nRate: {:.0} attempts/s\n",
                                    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
                                    rule.raw.as_str(),
                                    pub58_str,
                                    public_bytes,
                                    secret_hex,
                                    secret_bytes,
                                    secret_json,
                                    total_attempts,
                                    rate
                                );
                                let result_file = next_result_path(rule.raw.as_str());
                                append_block(result_file.as_str(), &summary);
                            }
                            return;
                        }
                    }

                    if local_attempts > 0 {
                        attempts.fetch_add(local_attempts, Ordering::Relaxed);
                    }
                });
            }
        });

        let _ = log_handle.join();
        matches_found += 1;
    }

    println!("Finished after {} matches.", matches_found);
}

fn format_with_commas(value: u64) -> String {
    let s = value.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let len = s.len();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn next_result_path(prefix: &str) -> String {
    for idx in 1u64.. {
        let path = format!("vanity_result_{}_{}.txt", prefix, idx);
        if !Path::new(&path).exists() {
            return path;
        }
    }
    unreachable!("Iterator over u64 exhausted");
}

fn append_block(path: &str, block: &str) {
    if let Err(err) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            file.write_all(block.as_bytes())?;
            file.write_all(b"\n")?;
            Ok(())
        })
    {
        eprintln!("Failed to write to {}: {}", path, err);
    }
}

#[derive(Clone)]
struct PrefixRule {
    raw: String,
    bytes: Vec<u8>,
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

fn candidate_matches(candidate: &[u8], prefix: &[u8], ignore_case: bool) -> bool {
    if candidate.len() < prefix.len() {
        return false;
    }
    if !ignore_case {
        candidate[..prefix.len()] == prefix[..]
    } else {
        candidate[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| eq_ignore_ascii_case(*a, *b))
    }
}

fn eq_ignore_ascii_case(a: u8, b: u8) -> bool {
    a == b || a.to_ascii_lowercase() == b.to_ascii_lowercase()
}
