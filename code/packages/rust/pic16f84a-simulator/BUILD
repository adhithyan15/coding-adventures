cargo test --locked -p pic16f84a-simulator --release --no-fail-fast
cargo clippy --locked -p pic16f84a-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pic16f84a-simulator --no-deps
