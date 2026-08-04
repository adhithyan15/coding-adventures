# build-tool: deps=rust/sha256-c
cargo build --manifest-path ../../rust/Cargo.toml -p sha256-c --release && cp ../../rust/target/release/libsha256_c.a Sources/CSha256/ && swift test
