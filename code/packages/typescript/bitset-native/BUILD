npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/bitset_native_node.node . 2>/dev/null || cp target/release/libbitset_native_node.so bitset_native_node.node 2>/dev/null || cp target/release/libbitset_native_node.dylib bitset_native_node.node 2>/dev/null || cp target/release/bitset_native_node.dll bitset_native_node.node 2>/dev/null
npx vitest run --coverage
