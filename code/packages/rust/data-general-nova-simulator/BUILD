cargo test --locked -p data-general-nova-simulator --release --all-targets
cargo clippy --locked -p data-general-nova-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p data-general-nova-simulator --no-deps
