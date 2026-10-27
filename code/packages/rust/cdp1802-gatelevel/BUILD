cargo test -p cdp1802-gatelevel --release --no-fail-fast
cargo clippy -p cdp1802-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdp1802-gatelevel --no-deps
