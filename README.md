CPU-only Solana vanity address miner built on top of `ed25519-dalek`
and Solana CLI-compatible key dumps (base58, hex, byte array, JSON),
with multi-threaded brute force and live stats.

```
RUSTFLAGS="-C target-cpu=native" cargo run --release -- --prefix xyber,wotori --max-matches 0 --ignore-case
```