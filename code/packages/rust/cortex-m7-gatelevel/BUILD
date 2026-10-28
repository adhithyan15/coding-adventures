cargo test -p cortex-m7-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m7-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m7-gatelevel --no-deps
