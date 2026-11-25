cargo test --locked -p cdp1802-gatelevel --release --no-fail-fast
cargo clippy --locked -p cdp1802-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cdp1802-gatelevel --no-deps
