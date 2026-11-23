cargo test --locked --release -p ias-gatelevel
cargo clippy --locked -p ias-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ias-gatelevel --no-deps
