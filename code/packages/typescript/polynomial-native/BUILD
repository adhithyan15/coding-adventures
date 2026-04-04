npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/polynomial_native_node.node . 2>/dev/null || cp target/release/libpolynomial_native_node.so polynomial_native_node.node 2>/dev/null || cp target/release/libpolynomial_native_node.dylib polynomial_native_node.node 2>/dev/null || cp target/release/polynomial_native_node.dll polynomial_native_node.node 2>/dev/null
npx vitest run --coverage
