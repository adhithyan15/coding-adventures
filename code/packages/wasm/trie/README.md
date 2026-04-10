# trie-wasm

WebAssembly bindings for the Rust [trie](../../rust/trie/) crate.

## What It Provides

- `WasmTrie` for string keys and string values in JavaScript
- Exact lookup, prefix lookup, deletion, and validation helpers
- Prefix result exports as JavaScript arrays of `[key, value]` pairs

## Usage

```javascript
import init, { WasmTrie } from "./trie_wasm.js";

await init();

const trie = new WasmTrie();
trie.insert("app", "short");
trie.insert("apple", "fruit");

trie.search("apple");            // "fruit"
trie.startsWith("app");          // true
trie.wordsWithPrefix("app");     // [["app", "short"], ["apple", "fruit"]]
trie.longestPrefixMatch("applepie"); // ["apple", "fruit"]
```

## Building

```bash
cargo test
wasm-pack build --target web
```
