python3 tools/verify_oracle_vectors.py
cargo test -p zpu-simulator --release --no-fail-fast
cargo clippy -p zpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p zpu-simulator --no-deps
