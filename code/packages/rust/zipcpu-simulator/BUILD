cargo test --locked -p zipcpu-simulator --release --no-fail-fast
cargo clippy --locked -p zipcpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p zipcpu-simulator --no-deps
