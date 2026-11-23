cargo test --locked -p pa-risc11-gatelevel --release --no-fail-fast
cargo clippy --locked -p pa-risc11-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pa-risc11-gatelevel --no-deps
