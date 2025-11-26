use chrono::{SecondsFormat, Utc};
use clap::Parser;
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
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
    /// Desired prefix for the base58-encoded public key
    #[arg(long, default_value = "xyber")]
    prefix: String,

    /// Number of matches to find before exiting (0 = run forever)
    #[arg(long = "max-matches", default_value_t = 1)]
    max_matches: u64,
}

fn main() {
    let args = Args::parse();
    let prefix = Arc::new(args.prefix);
    let max_matches = args.max_matches;
    let threads = num_cpus::get();
    println!("Using {} threads", threads);
    println!("Searching for prefix: {}", prefix.as_str());
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
        let result_path = Arc::new(next_result_path(prefix.as_str()));
        println!("Run #{} results file: {}", run_index, result_path.as_str());

        let logger_attempts = attempts.clone();
        let logger_found = found.clone();
        let logger_start = start;
        let log_handle = thread::spawn(move || {
            let interval = Duration::from_secs(5);
            loop {
                thread::sleep(interval);
                let total = logger_attempts.load(Ordering::Relaxed);
                let elapsed = logger_start.elapsed().as_secs_f64().max(1.0);
                let rate = (total as f64 / elapsed).round() as u64;
                let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
                let log_line = format!(
                    "[{}] Total attempts: {}  |  Rate: {} attempts/s",
                    timestamp,
                    format_with_commas(total),
                    format_with_commas(rate)
                );
                println!("{}", log_line);
                append_block("vanity_progress.log", &log_line);

                if logger_found.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        let prefix_for_threads = prefix.clone();
        scope(|s| {
            for _ in 0..threads {
                let prefix = prefix_for_threads.clone();
                let found = found.clone();
                let attempts = attempts.clone();
                let result_path = result_path.clone();
                let start = start;

                s.spawn(move |_| {
                    let mut rng = OsRng {};
                    while !found.load(Ordering::Relaxed) {
                        let kp = Keypair::generate(&mut rng);
                        attempts.fetch_add(1, Ordering::Relaxed);

                        let pub58 = bs58::encode(kp.public.to_bytes()).into_string();

                        if pub58.starts_with(prefix.as_str()) {
                            if !found.swap(true, Ordering::Relaxed) {
                                println!("Found match!");
                                println!("Public key: {}", pub58);
                                // kp.to_bytes() returns 64 bytes: secret + public
                                let secret_full = kp.to_bytes();
                                let secret_hex = hex::encode(secret_full);
                                println!("Secret key (hex): {}", secret_hex);
                                let elapsed = start.elapsed().as_secs_f64();
                                println!(
                                    "Attempts: {} ({:.0} / sec)",
                                    attempts.load(Ordering::Relaxed),
                                    attempts.load(Ordering::Relaxed) as f64 / elapsed
                                );

                                let rate =
                                    (attempts.load(Ordering::Relaxed) as f64 / elapsed).round();
                                let secret_bytes = format!("{:?}", secret_full);
                                let public_bytes = format!("{:?}", kp.public.to_bytes());
                                let secret_json = serde_json::to_string(&secret_full.to_vec())
                                    .unwrap_or_else(|_| "[]".to_string());
                                let summary = format!(
                                    "[{}] Prefix: {}\nPublic key: {}\nPublic key (bytes): {}\nSecret key (hex): {}\nSecret key (bytes): {}\nSecret key (json array): {}\nAttempts: {}\nRate: {:.0} attempts/s\n",
                                    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
                                    prefix.as_str(),
                                    pub58,
                                    public_bytes,
                                    secret_hex,
                                    secret_bytes,
                                    secret_json,
                                    attempts.load(Ordering::Relaxed),
                                    rate
                                );
                                append_block(result_path.as_str(), &summary);
                            }
                            return;
                        }
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
