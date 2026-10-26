cargo test -p latticemico32-simulator --release --no-fail-fast
cargo clippy -p latticemico32-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p latticemico32-simulator --no-deps
