cargo test --locked --release -p vax11-780-gatelevel
cargo clippy --locked -p vax11-780-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p vax11-780-gatelevel --no-deps
