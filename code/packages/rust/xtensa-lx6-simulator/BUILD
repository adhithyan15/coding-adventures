cargo test --locked -p xtensa-lx6-simulator --release --no-fail-fast
cargo clippy --locked -p xtensa-lx6-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p xtensa-lx6-simulator --no-deps
