# heap-wasm

WebAssembly bindings for the Rust [heap](../../rust/heap/) crate.

## What It Provides

- `WasmMinHeap` and `WasmMaxHeap` for integer workloads in JS
- `heapify`, `heapSort`, `nlargest`, and `nsmallest` exported to JavaScript
- Shared implementation logic from the Rust `heap` crate underneath

## Usage

```javascript
import init, { WasmMinHeap, heapSort } from "./heap_wasm.js";

await init();

const heap = WasmMinHeap.fromValues([5, 3, 8, 1, 4]);
heap.push(0);
heap.peek();      // 0
heap.pop();       // 0
heap.toArray();   // heap layout as an array

heapSort([3, 1, 4, 1, 5]); // [1, 1, 3, 4, 5]
```

## Building

```bash
cargo test
wasm-pack build --target web
```
