cargo test -p ias-simulator --no-fail-fast
cargo clippy -p ias-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ias-simulator --no-deps
