cargo test --locked -p superh2-simulator --release --no-fail-fast
cargo clippy --locked -p superh2-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p superh2-simulator --no-deps
