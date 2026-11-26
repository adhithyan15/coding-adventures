cargo test --locked -p pdp8-simulator --no-fail-fast
cargo clippy --locked -p pdp8-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pdp8-simulator --no-deps
