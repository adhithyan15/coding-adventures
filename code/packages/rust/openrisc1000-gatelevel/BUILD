cargo test --locked -p openrisc1000-gatelevel --release --no-fail-fast
cargo clippy --locked -p openrisc1000-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p openrisc1000-gatelevel --no-deps
