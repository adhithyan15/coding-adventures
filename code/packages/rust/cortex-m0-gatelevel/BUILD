cargo test -p cortex-m0-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m0-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m0-gatelevel --no-deps
