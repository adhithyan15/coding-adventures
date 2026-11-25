cargo test --locked --manifest-path Cargo.toml
cargo clippy --locked --manifest-path Cargo.toml --all-targets -- -D warnings
python ../../../tests/rust-guest-microbenchmark-consumer/native_acceptance.py --package Cargo.toml --bin starlark-microbenchmark --guest starlark-vm --workload starlark-bytecode-integer-add-chain-256
