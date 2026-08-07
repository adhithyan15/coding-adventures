# build-tool: deps=rust/md5-c
cargo build --manifest-path ../../rust/Cargo.toml -p md5-c --release && cp ../../rust/target/release/libmd5_c.a Sources/CMd5/ && swift test
