cargo test --release -p ias-gatelevel
cargo clippy -p ias-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ias-gatelevel --no-deps
