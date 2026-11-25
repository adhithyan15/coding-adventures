cargo test --locked -p cortex-m33-simulator --release
cargo clippy --locked -p cortex-m33-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m33-simulator --no-deps
