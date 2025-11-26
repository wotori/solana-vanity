use rand_chacha::ChaCha20Rng;

use crate::matcher::PrefixRule;

pub struct BackendMatch {
    pub prefix: String,
    pub public_str: String,
    pub public_bytes: Vec<u8>,
    pub secret_hex: String,
    pub secret_bytes: Vec<u8>,
    pub secret_json: String,
    pub secret_wif: Option<String>,
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
