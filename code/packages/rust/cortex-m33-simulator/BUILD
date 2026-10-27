cargo test -p cortex-m33-simulator --release
cargo clippy -p cortex-m33-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m33-simulator --no-deps
