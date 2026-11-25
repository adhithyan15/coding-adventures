cargo test --locked -p avr-classic-simulator --release --no-fail-fast
cargo clippy --locked -p avr-classic-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p avr-classic-simulator --no-deps
