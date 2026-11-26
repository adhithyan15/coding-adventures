cargo test --locked -p arm7tdmi-gatelevel --release --no-fail-fast
cargo clippy --locked -p arm7tdmi-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p arm7tdmi-gatelevel --no-deps
