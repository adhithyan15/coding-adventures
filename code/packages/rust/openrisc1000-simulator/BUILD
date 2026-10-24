cargo test -p openrisc1000-simulator --release --no-fail-fast
cargo clippy -p openrisc1000-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p openrisc1000-simulator --no-deps

