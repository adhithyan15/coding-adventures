cargo test --locked -p pdp11-gatelevel --no-fail-fast
cargo clippy --locked -p pdp11-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pdp11-gatelevel --no-deps
