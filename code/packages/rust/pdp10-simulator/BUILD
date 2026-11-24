cargo test --locked -p pdp10-simulator --no-fail-fast
cargo clippy --locked -p pdp10-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pdp10-simulator --no-deps
