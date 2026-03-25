npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/commonmark_native_node.node . 2>/dev/null || cp target/release/libcommonmark_native_node.so commonmark_native_node.node 2>/dev/null || cp target/release/libcommonmark_native_node.dylib commonmark_native_node.node 2>/dev/null || cp target/release/commonmark_native_node.dll commonmark_native_node.node 2>/dev/null
npx vitest run --coverage
