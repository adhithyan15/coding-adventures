cargo test -p cortex-m4-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m4-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m4-gatelevel --no-deps
