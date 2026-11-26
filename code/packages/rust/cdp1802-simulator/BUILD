cargo test --locked -p cdp1802-simulator --release --no-fail-fast
cargo clippy --locked -p cdp1802-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cdp1802-simulator --no-deps
