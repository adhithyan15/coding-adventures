cargo test -p avr-classic-simulator --release --no-fail-fast
cargo clippy -p avr-classic-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p avr-classic-simulator --no-deps

