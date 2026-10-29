cargo test -p motorola68020-simulator --release --no-fail-fast
cargo clippy -p motorola68020-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p motorola68020-simulator --no-deps
