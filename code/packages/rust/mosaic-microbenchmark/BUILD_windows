cargo test --locked --manifest-path Cargo.toml
cargo clippy --locked --manifest-path Cargo.toml --all-targets -- -D warnings
python ../../../tests/rust-guest-microbenchmark-consumer/native_acceptance.py --package Cargo.toml --bin mosaic-microbenchmark --guest mosaic-vm --workload mosaic-ir-tree-walk-event-stream-4096
