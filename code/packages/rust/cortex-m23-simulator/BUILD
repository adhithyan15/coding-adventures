cargo test --locked -p cortex-m23-simulator --release
cargo clippy --locked -p cortex-m23-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m23-simulator --no-deps
