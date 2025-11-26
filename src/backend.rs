use ed25519_dalek::Keypair;
use rand_chacha::ChaCha20Rng;

use crate::matcher::{candidate_matches, PrefixRule};

pub struct BackendMatch {
    pub prefix: String,
    pub public_str: String,
    pub public_bytes: Vec<u8>,
    pub secret_hex: String,
    pub secret_bytes: Vec<u8>,
    pub secret_json: String,
}

pub trait VanityBackend: Send + Sync {
    fn name(&self) -> &'static str;

    fn try_generate_match(
        &self,
        rng: &mut ChaCha20Rng,
        rules: &[PrefixRule],
        ignore_case: bool,
        buffer: &mut [u8],
    ) -> Option<BackendMatch>;
}

pub struct SolanaBackend;

impl VanityBackend for SolanaBackend {
    fn name(&self) -> &'static str {
        "solana"
    }

    fn try_generate_match(
        &self,
        rng: &mut ChaCha20Rng,
        rules: &[PrefixRule],
        ignore_case: bool,
        buffer: &mut [u8],
    ) -> Option<BackendMatch> {
        let keypair = Keypair::generate(rng);
        let len = bs58::encode(keypair.public.to_bytes())
            .onto(&mut *buffer)
            .expect("base58 buffer too small");
        let candidate = &buffer[..len];

        let rule = rules
            .iter()
            .find(|rule| candidate_matches(candidate, &rule.bytes, ignore_case))?;

        let public_str = std::str::from_utf8(candidate)
            .expect("base58 output should be valid UTF-8")
            .to_string();

        let secret_full = keypair.to_bytes();
        let secret_hex = hex::encode(secret_full);
        let secret_bytes = secret_full.to_vec();
        let public_bytes = keypair.public.to_bytes().to_vec();
        let secret_json = serde_json::to_string(&secret_bytes).unwrap_or_else(|_| "[]".into());

        Some(BackendMatch {
            prefix: rule.raw.clone(),
            public_str,
            public_bytes,
            secret_hex,
            secret_bytes,
            secret_json,
        })
    }
}
