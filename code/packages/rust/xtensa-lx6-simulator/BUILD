cargo test -p xtensa-lx6-simulator --release --no-fail-fast
cargo clippy -p xtensa-lx6-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p xtensa-lx6-simulator --no-deps
