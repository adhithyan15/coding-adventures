cargo test -p pdp10-simulator --no-fail-fast
cargo clippy -p pdp10-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pdp10-simulator --no-deps
