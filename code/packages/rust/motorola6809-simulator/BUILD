cargo test --locked -p motorola6809-simulator --release --no-fail-fast
cargo clippy --locked -p motorola6809-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p motorola6809-simulator --no-deps
