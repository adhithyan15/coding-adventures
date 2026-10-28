cargo test -p cortex-m23-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m23-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m23-gatelevel --no-deps
