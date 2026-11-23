python3 tools/verify_oracle_vectors.py
cargo test --locked -p zpu-simulator --release --no-fail-fast
cargo clippy --locked -p zpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p zpu-simulator --no-deps
