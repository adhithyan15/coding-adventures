cargo test --locked -p cortex-m4-gatelevel --release --no-fail-fast
cargo clippy --locked -p cortex-m4-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cortex-m4-gatelevel --no-deps
