CPU-only vanity address miner with a pluggable backend (Solana by default,
Bitcoin via `--chain bitcoin`). Uses multi-threaded brute force, live stats,
and dumps keys in multiple formats (base58, hex, byte array, JSON).

- `--chain solana` (default) keeps the original ed25519 flow.
- `--chain bitcoin` switches to a secp256k1 backend and supports:
  - `--type legacy` (default): legacy base58 `1...` addresses.
  - `--type segwit`: native SegWit `bc1q...` addresses.

```
RUSTFLAGS="-C target-cpu=native" cargo run --release -- --prefix xyber,wotori --max-matches 0 --ignore-case --chain solana
```