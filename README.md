CPU-only Solana vanity address miner built on top of `ed25519-dalek`
and Solana CLI-compatible key dumps (base58, hex, byte array, JSON),
with multi-threaded brute force and live stats.

```
cargo run --release -- --prefix ekza --max-matches 0
```