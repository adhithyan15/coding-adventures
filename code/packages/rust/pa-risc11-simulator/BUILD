cargo test --locked -p pa-risc11-simulator --release --no-fail-fast
cargo clippy --locked -p pa-risc11-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pa-risc11-simulator --no-deps
