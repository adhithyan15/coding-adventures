cargo test --locked -p opensparc-t1-gatelevel --release --no-fail-fast
cargo clippy --locked -p opensparc-t1-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p opensparc-t1-gatelevel --no-deps
