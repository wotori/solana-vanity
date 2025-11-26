use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};

use crate::miner::VanityResult;

pub fn print_and_save_result(result: &VanityResult) {
    let pub58_str = bs58::encode(result.keypair.public.to_bytes()).into_string();
    let secret_full = result.keypair.to_bytes();
    let secret_hex = hex::encode(secret_full);
    let secret_bytes = format!("{secret_full:?}");
    let public_bytes = format!("{:?}", result.keypair.public.to_bytes());
    let secret_json = serde_json::to_string(&secret_full.to_vec()).unwrap_or_else(|_| "[]".into());

    println!("Found match!");
    println!("Prefix: {}", result.prefix);
    println!("Public key: {pub58_str}");
    println!("Secret key (hex): {secret_hex}");
    println!(
        "Attempts: {} ({:.0} / sec)",
        result.stats.attempts, result.stats.rate
    );
    println!("Elapsed: {:.2} sec", result.stats.elapsed_secs);

    let summary = format!(
        "[{}] Prefix: {}\n\
         Public key: {}\n\
         Public key (bytes): {}\n\
         Secret key (hex): {}\n\
         Secret key (bytes): {}\n\
         Secret key (json array): {}\n\
         Attempts: {}\n\
         Elapsed: {:.2} sec\n\
         Rate: {:.0} attempts/s\n",
        Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
        result.prefix,
        pub58_str,
        public_bytes,
        secret_hex,
        secret_bytes,
        secret_json,
        result.stats.attempts,
        result.stats.elapsed_secs,
        result.stats.rate,
    );

    let result_file = next_result_path(&result.prefix);
    append_block(&result_file, &summary);
}

fn next_result_path(prefix: &str) -> String {
    for idx in 1u64.. {
        let path = format!("vanity_result_{prefix}_{idx}.txt");
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
        eprintln!("Failed to write to {path}: {err}");
    }
}
