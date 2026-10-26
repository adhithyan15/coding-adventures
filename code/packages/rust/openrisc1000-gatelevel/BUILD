cargo test -p openrisc1000-gatelevel --release --no-fail-fast
cargo clippy -p openrisc1000-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p openrisc1000-gatelevel --no-deps
