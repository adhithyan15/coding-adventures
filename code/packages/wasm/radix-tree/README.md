# radix-tree-wasm

WebAssembly bindings for the Rust [radix-tree](../../rust/radix-tree/) crate.

## What It Provides

- `WasmRadixTree` for string keys and string values in JavaScript
- Compressed-prefix lookup, deletion, and node-count inspection
- Prefix result exports as JavaScript arrays, plus `toMap()` for key/value materialization

## Usage

```javascript
import init, { WasmRadixTree } from "./radix_tree_wasm.js";

await init();

const tree = new WasmRadixTree();
tree.insert("app", "short");
tree.insert("apple", "fruit");

tree.search("apple");                // "fruit"
tree.wordsWithPrefix("app");         // ["app", "apple"]
tree.longestPrefixMatch("applepie"); // "apple"
tree.toMap();                        // Map { "app" => "short", "apple" => "fruit" }
```

## Building

```bash
cargo test
wasm-pack build --target web
```
