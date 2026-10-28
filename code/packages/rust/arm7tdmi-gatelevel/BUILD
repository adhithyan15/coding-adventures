cargo test -p arm7tdmi-gatelevel --release --no-fail-fast
cargo clippy -p arm7tdmi-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p arm7tdmi-gatelevel --no-deps
