use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use ed25519_dalek::Keypair;
use rand::{rngs::OsRng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::config::Config;
use crate::matcher::{candidate_matches, PrefixRule};

#[derive(Clone, Debug)]
pub struct MiningStats {
    pub attempts: u64,
    pub elapsed_secs: f64,
    pub rate: f64,
}

pub struct VanityResult {
    pub prefix: String,
    pub keypair: Keypair,
    pub stats: MiningStats,
}

pub fn mine_one_round(config: &Config) -> VanityResult {
    let found = Arc::new(AtomicBool::new(false));
    let attempts = Arc::new(AtomicU64::new(0));
    let result = Arc::new(Mutex::new(None::<VanityResult>));

    let start = Instant::now();
    let log_handle = spawn_logger(attempts.clone(), found.clone(), start);

    let rules = config.prefix_rules.clone();
    let ignore_case = config.ignore_case;

    thread::scope(|scope| {
        for _ in 0..config.threads {
            let found = found.clone();
            let attempts = attempts.clone();
            let rules = rules.clone();
            let result = result.clone();

            scope.spawn(move || {
                worker_loop(found, attempts, rules, ignore_case, start, result);
            });
        }
    });

    let _ = log_handle.join();

    let total_attempts = attempts.load(Ordering::Relaxed);
    let elapsed = start.elapsed().as_secs_f64().max(1e-9);
    let rate = total_attempts as f64 / elapsed;

    let mut guard = result.lock().expect("result mutex poisoned");
    let mut vr = guard
        .take()
        .expect("worker should have set a VanityResult when found is true");

    vr.stats = MiningStats {
        attempts: total_attempts,
        elapsed_secs: elapsed,
        rate,
    };

    vr
}

fn spawn_logger(
    attempts: Arc<AtomicU64>,
    found: Arc<AtomicBool>,
    start: Instant,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let interval = Duration::from_secs(1);
        loop {
            thread::sleep(interval);
            let total = attempts.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_secs_f64().max(1.0);
            let rate = (total as f64 / elapsed).round() as u64;
            let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
            println!(
                "[{}] Total attempts: {}  |  Rate: {} attempts/s",
                timestamp,
                format_with_commas(total),
                format_with_commas(rate)
            );

            if found.load(Ordering::Relaxed) {
                break;
            }
        }
    })
}

fn worker_loop(
    found: Arc<AtomicBool>,
    attempts: Arc<AtomicU64>,
    rules: Arc<Vec<PrefixRule>>,
    ignore_case: bool,
    start: Instant,
    result: Arc<Mutex<Option<VanityResult>>>,
) {
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
            .expect("base58 buffer too small");
        let candidate = &buffer[..len];

        if let Some(rule) = rules
            .iter()
            .find(|rule| candidate_matches(candidate, &rule.bytes, ignore_case))
        {
            if !found.swap(true, Ordering::Relaxed) {
                if local_attempts > 0 {
                    attempts.fetch_add(local_attempts, Ordering::Relaxed);
                }

                let elapsed = start.elapsed().as_secs_f64().max(1e-9);
                let total_attempts = attempts.load(Ordering::Relaxed);
                let rate = total_attempts as f64 / elapsed;

                let stats = MiningStats {
                    attempts: total_attempts,
                    elapsed_secs: elapsed,
                    rate,
                };

                let vr = VanityResult {
                    prefix: rule.raw.clone(),
                    keypair: kp,
                    stats,
                };

                let mut guard = result.lock().expect("result mutex poisoned");
                *guard = Some(vr);
            }
            return;
        }
    }

    if local_attempts > 0 {
        attempts.fetch_add(local_attempts, Ordering::Relaxed);
    }
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
