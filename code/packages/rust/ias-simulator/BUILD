cargo test --locked -p ias-simulator --no-fail-fast
cargo clippy --locked -p ias-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ias-simulator --no-deps
