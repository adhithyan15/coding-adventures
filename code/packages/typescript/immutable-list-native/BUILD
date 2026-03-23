npm ci --quiet
cargo build --release --manifest-path Cargo.toml
cp target/release/immutable_list_native_node.node . 2>/dev/null || cp target/release/libimmutable_list_native_node.so immutable_list_native_node.node 2>/dev/null || cp target/release/libimmutable_list_native_node.dylib immutable_list_native_node.node 2>/dev/null || cp target/release/immutable_list_native_node.dll immutable_list_native_node.node 2>/dev/null
npx vitest run --coverage
