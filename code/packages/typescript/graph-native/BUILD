npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/graph_native_node.node . 2>/dev/null || cp target/release/libgraph_native_node.so graph_native_node.node 2>/dev/null || cp target/release/libgraph_native_node.dylib graph_native_node.node 2>/dev/null || cp target/release/graph_native_node.dll graph_native_node.node 2>/dev/null
npx vitest run --coverage
