cargo test --locked -p pdp11-simulator --no-fail-fast
cargo clippy --locked -p pdp11-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pdp11-simulator --no-deps
