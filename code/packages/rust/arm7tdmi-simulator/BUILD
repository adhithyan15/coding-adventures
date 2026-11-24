cargo test --locked -p arm7tdmi-simulator --release --no-fail-fast
cargo clippy --locked -p arm7tdmi-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p arm7tdmi-simulator --no-deps
