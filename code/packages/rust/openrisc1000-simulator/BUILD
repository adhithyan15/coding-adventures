cargo test --locked -p openrisc1000-simulator --release --no-fail-fast
cargo clippy --locked -p openrisc1000-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p openrisc1000-simulator --no-deps
