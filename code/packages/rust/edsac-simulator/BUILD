cargo test --locked -p edsac-simulator --no-fail-fast
cargo clippy --locked -p edsac-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p edsac-simulator --no-deps
