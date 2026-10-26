cargo test -p opensparc-t1-gatelevel --release --no-fail-fast
cargo clippy -p opensparc-t1-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p opensparc-t1-gatelevel --no-deps
