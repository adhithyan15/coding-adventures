cargo test --locked -p msp430-simulator --release
cargo clippy --locked -p msp430-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p msp430-simulator --no-deps
