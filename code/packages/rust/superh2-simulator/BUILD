cargo test -p superh2-simulator --release --no-fail-fast
cargo clippy -p superh2-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p superh2-simulator --no-deps
