use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};

use crate::miner::VanityResult;

pub fn print_and_save_result(result: &VanityResult) {
    let bm = &result.backend_match;
    let public_bytes = format!("{:?}", bm.public_bytes);
    let secret_bytes = format!("{:?}", bm.secret_bytes);

    println!("Found match!");
    println!("Backend: {}", result.backend_name);
    println!("Prefix: {}", bm.prefix);
    println!("Address / public: {}", bm.public_str);
    println!("Secret key (hex): {}", bm.secret_hex);
    println!(
        "Attempts: {} ({:.0} / sec)",
        result.stats.attempts, result.stats.rate
    );
    println!("Elapsed: {:.2} sec", result.stats.elapsed_secs);

    let summary = format!(
        "[{}] Backend: {}\n\
         Prefix: {}\n\
         Public: {}\n\
         Public (bytes): {}\n\
         Secret (hex): {}\n\
         Secret (bytes): {}\n\
         Secret (json array): {}\n\
         Attempts: {}\n\
         Elapsed: {:.2} sec\n\
         Rate: {:.0} attempts/s\n",
        Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
        result.backend_name,
        bm.prefix,
        bm.public_str,
        public_bytes,
        bm.secret_hex,
        secret_bytes,
        bm.secret_json,
        result.stats.attempts,
        result.stats.elapsed_secs,
        result.stats.rate,
    );

    let result_file = next_result_path(&bm.prefix);
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
