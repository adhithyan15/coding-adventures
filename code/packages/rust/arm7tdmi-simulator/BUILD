cargo test -p arm7tdmi-simulator --release --no-fail-fast
cargo clippy -p arm7tdmi-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p arm7tdmi-simulator --no-deps
