cargo test -p zipcpu-simulator --release --no-fail-fast
cargo clippy -p zipcpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p zipcpu-simulator --no-deps
