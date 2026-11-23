cargo test --locked -p pdp8-gatelevel --no-fail-fast
cargo clippy --locked -p pdp8-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pdp8-gatelevel --no-deps
