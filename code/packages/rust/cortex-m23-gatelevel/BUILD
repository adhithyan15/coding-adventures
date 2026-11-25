cargo test --locked -p cortex-m23-gatelevel --release --no-fail-fast
cargo clippy --locked -p cortex-m23-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m23-gatelevel --no-deps
