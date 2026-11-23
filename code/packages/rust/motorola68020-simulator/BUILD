cargo test --locked -p motorola68020-simulator --release --no-fail-fast
cargo clippy --locked -p motorola68020-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p motorola68020-simulator --no-deps
