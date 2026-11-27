cargo fmt --package arm1-gatelevel -- --check
cargo clippy --locked -p arm1-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p arm1-gatelevel --no-deps
cargo test --locked -p arm1-gatelevel --release --verbose
