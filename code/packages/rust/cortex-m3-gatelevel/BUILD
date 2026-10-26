cargo test -p cortex-m3-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m3-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m3-gatelevel --no-deps
