cargo test -p cdp1802-simulator --release --no-fail-fast
cargo clippy -p cdp1802-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdp1802-simulator --no-deps
