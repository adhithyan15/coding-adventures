cargo test -p pa-risc11-simulator --release --no-fail-fast
cargo clippy -p pa-risc11-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pa-risc11-simulator --no-deps
