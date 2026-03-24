# immutable-list-wasm

WebAssembly bindings for the [immutable-list](../../rust/immutable-list/) library, exposing a persistent vector with structural sharing for use in browsers and Deno.

## What This Does

This package wraps the pure-Rust `immutable-list` crate with `wasm-bindgen` so it can be called from JavaScript/TypeScript in browser or server-side WASM runtimes. Every "mutation" returns a **new** list -- the original is never modified. Under the hood, structural sharing (via 32-way tries and Arc pointers) keeps this efficient: a push typically copies only a small tail buffer.

## How It Fits in the Stack

```
+----------------------------+
|   Browser / Deno / Node    |  <-- JavaScript consumer
+----------------------------+
|   immutable-list-wasm      |  <-- wasm-bindgen wrapper (cdylib)
+----------------------------+
|   immutable-list (Rust)    |  <-- core logic, zero deps
+----------------------------+
```

The core `immutable-list` crate has zero external dependencies (only `std::sync::Arc`) and contains all the logic. This crate adds only `wasm-bindgen` and `js-sys` for the JS interop glue.

## Building

```bash
# Native unit tests (no WASM tooling needed):
cargo test

# Build WASM for browsers:
wasm-pack build --target web

# Build WASM for Node.js/Deno:
wasm-pack build --target nodejs
```

## Usage (JavaScript)

```javascript
import init, { WasmImmutableList } from './immutable_list_wasm.js';

await init();  // initialize WASM module

// Create a list by chaining pushes (each returns a new list)
const empty = new WasmImmutableList();
const list1 = empty.push("hello");
const list2 = list1.push("world");

empty.length();   // 0  -- unchanged
list1.length();   // 1  -- unchanged
list2.length();   // 2

list2.get(0);     // "hello"
list2.get(1);     // "world"
list2.get(99);    // undefined

// Update an element (returns new list)
const list3 = list2.set(0, "hi");
list3.get(0);     // "hi"
list2.get(0);     // "hello" -- unchanged

// Pop the last element
const [shorter, value] = list2.pop();
value;              // "world"
shorter.length();   // 1
list2.length();     // 2 -- unchanged

new WasmImmutableList().pop();  // null (empty list)

// Convert to/from JS arrays
list2.toArray();  // ["hello", "world"]
const fromArr = WasmImmutableList.fromArray(["a", "b", "c"]);
fromArr.length(); // 3
```

## API Reference

### Constructor

| Method | Description |
|--------|-------------|
| `new WasmImmutableList()` | Create an empty persistent list |

### Persistent Operations (return new lists)

| Method | Description |
|--------|-------------|
| `push(value)` | Append a string, returns new list |
| `set(index, value)` | Replace element at index, returns new list |
| `pop()` | Returns `[new_list, removed_value]` or `null` if empty |

### Queries

| Method | Description |
|--------|-------------|
| `get(index)` | Returns the string at index, or `undefined` |
| `length()` | Number of elements |
| `isEmpty()` | True if length is 0 |

### Conversion

| Method | Description |
|--------|-------------|
| `toArray()` | JS array of all strings |
| `WasmImmutableList.fromArray(arr)` | Construct from a JS string array |

## Why Persistent?

Persistent data structures are ideal for:

- **Undo/redo**: keep references to previous versions at zero cost.
- **React/Redux state**: compare old vs. new state cheaply (reference equality).
- **Concurrent access**: no locks needed since data never changes.
- **Time-travel debugging**: every version of the state is preserved.

The 32-way trie ensures that push, get, set, and pop are all effectively O(1) for practical list sizes (the tree is at most 4-6 levels deep for millions of elements).
