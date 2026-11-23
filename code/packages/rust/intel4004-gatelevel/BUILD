cargo fmt --package intel4004-gatelevel -- --check
cargo clippy --locked -p intel4004-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p intel4004-gatelevel --no-deps
cargo test --locked -p intel4004-gatelevel --release --verbose
