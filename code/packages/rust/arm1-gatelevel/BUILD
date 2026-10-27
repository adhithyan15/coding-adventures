cargo fmt --package arm1-gatelevel -- --check
cargo clippy -p arm1-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p arm1-gatelevel --no-deps
cargo test -p arm1-gatelevel --release --verbose
