(cd ../pixel-container && npm ci --quiet)
(cd ../paint-instructions && npm ci --quiet)
(cd ../barcode-layout-1d && npm ci --quiet)
(cd ../codabar && npm ci --quiet)
(cd ../code128 && npm ci --quiet)
(cd ../code39 && npm ci --quiet)
(cd ../ean-13 && npm ci --quiet)
(cd ../itf && npm ci --quiet)
(cd ../upc-a && npm ci --quiet)
npm ci --quiet
cargo build --release --manifest-path ./Cargo.toml
cp target/release/barcode_1d_native_node.node . 2>/dev/null || cp target/release/libbarcode_1d_native_node.so barcode_1d_native_node.node 2>/dev/null || cp target/release/libbarcode_1d_native_node.dylib barcode_1d_native_node.node 2>/dev/null || cp target/release/barcode_1d_native_node.dll barcode_1d_native_node.node 2>/dev/null
node --test ./tests/barcode-1d.test.js
