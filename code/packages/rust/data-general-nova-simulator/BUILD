cargo test -p data-general-nova-simulator --release --all-targets
cargo clippy -p data-general-nova-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p data-general-nova-simulator --no-deps
