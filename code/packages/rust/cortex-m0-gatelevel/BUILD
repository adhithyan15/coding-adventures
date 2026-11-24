cargo test --locked -p cortex-m0-gatelevel --release --no-fail-fast
cargo clippy --locked -p cortex-m0-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m0-gatelevel --no-deps
