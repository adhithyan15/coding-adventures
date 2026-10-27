cargo test -p msp430-simulator --release
cargo clippy -p msp430-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p msp430-simulator --no-deps
