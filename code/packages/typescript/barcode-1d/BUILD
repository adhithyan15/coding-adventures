npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/barcode_1d_native_node.node . 2>/dev/null || cp target/release/libbarcode_1d_native_node.so barcode_1d_native_node.node 2>/dev/null || cp target/release/libbarcode_1d_native_node.dylib barcode_1d_native_node.node 2>/dev/null || cp target/release/barcode_1d_native_node.dll barcode_1d_native_node.node 2>/dev/null
npx vitest run --coverage
