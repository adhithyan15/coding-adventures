cargo fmt --package intel4004-gatelevel -- --check
cargo clippy -p intel4004-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p intel4004-gatelevel --no-deps
cargo test -p intel4004-gatelevel --release --verbose
