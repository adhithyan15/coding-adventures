# fenwick-tree-wasm

WebAssembly bindings for the Rust [fenwick-tree](../../rust/fenwick-tree/) crate.

## What It Provides

- `WasmFenwickTree` for prefix sums and point updates from JavaScript
- Order-statistics lookup with `findKth`
- Shared logic from the Rust `fenwick-tree` crate

## Usage

```javascript
import init, { WasmFenwickTree } from "./fenwick_tree_wasm.js";

await init();

const tree = WasmFenwickTree.fromValues([3, 2, 1, 7, 4]);
tree.prefixSum(3);      // 6
tree.rangeSum(2, 4);    // 10
tree.update(3, 5);
tree.pointQuery(3);     // 6
tree.findKth(11);       // 3
```

## Building

```bash
cargo test
wasm-pack build --target web
```
