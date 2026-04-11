npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/tree_set_native_node.node . 2>/dev/null || cp target/release/libtree_set_native_node.so tree_set_native_node.node 2>/dev/null || cp target/release/libtree_set_native_node.dylib tree_set_native_node.node 2>/dev/null || cp target/release/tree_set_native_node.dll tree_set_native_node.node 2>/dev/null
npx vitest run --coverage
