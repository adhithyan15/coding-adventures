cargo test -p pic16f84a-simulator --release --no-fail-fast
cargo clippy -p pic16f84a-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pic16f84a-simulator --no-deps
