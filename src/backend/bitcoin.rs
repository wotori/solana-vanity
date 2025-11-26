use bitcoin::address::KnownHrp;
use bitcoin::key::{CompressedPublicKey, PrivateKey};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::{Address, Network, PublicKey};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;

use crate::matcher::{candidate_matches, PrefixRule};

use super::interface::{BackendMatch, VanityBackend};

pub struct BitcoinBackend {
    network: Network,
    address_type: BitcoinAddressType,
}

impl BitcoinBackend {
    pub fn new(network: Network, address_type: BitcoinAddressType) -> Self {
        Self {
            network,
            address_type,
        }
    }
}

impl Default for BitcoinBackend {
    fn default() -> Self {
        Self::new(Network::Bitcoin, BitcoinAddressType::Legacy)
    }
}

#[derive(Copy, Clone)]
pub enum BitcoinAddressType {
    Legacy,
    Segwit,
}

impl VanityBackend for BitcoinBackend {
    fn name(&self) -> &'static str {
        "bitcoin"
    }

    fn try_generate_match(
        &self,
        rng: &mut ChaCha20Rng,
        rules: &[PrefixRule],
        ignore_case: bool,
        buffer: &mut [u8],
    ) -> Option<BackendMatch> {
        let secp = Secp256k1::new();
        let secret_key = loop {
            let mut candidate = [0u8; 32];
            rng.fill_bytes(&mut candidate);
            if let Ok(sk) = SecretKey::from_slice(&candidate) {
                break sk;
            }
        };
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let bitcoin_pk = PublicKey::new(public_key);
        let compressed_pk = CompressedPublicKey::try_from(bitcoin_pk).expect("compressed pubkey");
        let address = match self.address_type {
            BitcoinAddressType::Legacy => Address::p2pkh(bitcoin_pk, self.network),
            BitcoinAddressType::Segwit => {
                Address::p2wpkh(&compressed_pk, KnownHrp::from(self.network))
            }
        };
        let address_str = address.to_string();
        let address_bytes = address_str.as_bytes();
        if address_bytes.len() > buffer.len() {
            return None;
        }
        buffer[..address_bytes.len()].copy_from_slice(address_bytes);
        let candidate = &buffer[..address_bytes.len()];

        let rule = rules
            .iter()
            .find(|rule| candidate_matches(candidate, &rule.bytes, ignore_case))?;

        let secret_bytes = secret_key.secret_bytes();
        let public_bytes = bitcoin_pk.inner.serialize();
        let secret_vec = secret_bytes.to_vec();

        let private_key = PrivateKey {
            compressed: true,
            network: self.network.into(),
            inner: secret_key,
        };

        Some(BackendMatch {
            prefix: rule.raw.clone(),
            public_str: address_str,
            public_bytes: public_bytes.to_vec(),
            secret_hex: hex::encode(secret_bytes),
            secret_bytes: secret_vec.clone(),
            secret_json: serde_json::to_string(&secret_vec).unwrap_or_else(|_| "[]".into()),
            secret_wif: Some(private_key.to_wif()),
        })
    }
}
