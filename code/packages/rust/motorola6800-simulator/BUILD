cargo test -p motorola6800-simulator --release --no-fail-fast
cargo clippy -p motorola6800-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p motorola6800-simulator --no-deps
