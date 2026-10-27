cargo test -p pa-risc11-gatelevel --release --no-fail-fast
cargo clippy -p pa-risc11-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pa-risc11-gatelevel --no-deps
