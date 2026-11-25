cargo test --locked -p cortex-m7-gatelevel --release --no-fail-fast
cargo clippy --locked -p cortex-m7-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m7-gatelevel --no-deps
