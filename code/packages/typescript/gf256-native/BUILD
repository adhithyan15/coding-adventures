npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/gf256_native_node.node . 2>/dev/null || cp target/release/libgf256_native_node.so gf256_native_node.node 2>/dev/null || cp target/release/libgf256_native_node.dylib gf256_native_node.node 2>/dev/null || cp target/release/gf256_native_node.dll gf256_native_node.node 2>/dev/null
npx vitest run --coverage
