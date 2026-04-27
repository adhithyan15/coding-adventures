# directed-graph-wasm

WASM-compiled directed graph — runs in browsers, Node.js, Deno, and any WASM runtime.

## What is this?

This package compiles the Rust `directed-graph` crate to WebAssembly via wasm-bindgen. The result is a `.wasm` file + JavaScript glue code that works anywhere WASM is supported:

- **Browsers** — Chrome, Firefox, Safari, Edge
- **Node.js** — via the generated JS bindings
- **Deno** — native WASM support
- **Edge runtimes** — Cloudflare Workers, Vercel Edge Functions
- **Standalone** — wasmtime, wasmer (can be loaded from Python, Ruby, etc.)

## Usage

```javascript
import init, { DirectedGraph } from './directed_graph_wasm.js';

await init();  // load the WASM module

const g = new DirectedGraph();
g.addEdge("compile", "link");
g.addEdge("link", "package");

console.log(g.topologicalSort());    // ['compile', 'link', 'package']
console.log(g.independentGroups());  // [['compile'], ['link'], ['package']]
console.log(g.hasCycle());           // false

const affected = g.affectedNodes(["compile"]);
console.log(affected);               // ['compile', 'link', 'package']
```

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for browser
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs

# Build for bundlers (webpack, vite, etc.)
wasm-pack build --target bundler
```

## API

Methods use camelCase (JavaScript convention):

| Method | Description |
|--------|-------------|
| `addNode(name)` | Add a node |
| `removeNode(name)` | Remove a node and its edges |
| `hasNode(name)` | Check if node exists |
| `nodes()` | Sorted array of all nodes |
| `size()` | Number of nodes |
| `addEdge(from, to)` | Add directed edge |
| `removeEdge(from, to)` | Remove edge |
| `hasEdge(from, to)` | Check if edge exists |
| `edges()` | Sorted array of `[from, to]` pairs |
| `predecessors(node)` | Nodes pointing to this node |
| `successors(node)` | Nodes this node points to |
| `topologicalSort()` | Kahn's algorithm |
| `hasCycle()` | DFS 3-color cycle detection |
| `transitiveClosure(node)` | All reachable nodes |
| `affectedNodes(changed)` | Changed + transitive dependents |
| `independentGroups()` | Parallel execution levels |

## How it differs from native extensions

| | Native (PyO3/Magnus/napi-rs) | WASM |
|--|------------------------------|------|
| Performance | Best (zero overhead FFI) | Good (slight WASM overhead) |
| Platform | Need wheels per OS/arch | Universal binary |
| Build | Per-language tooling | One build, runs everywhere |
| Strings | Shared memory | Copied across boundary |

For a graph library, the WASM overhead is negligible — algorithms are the bottleneck, not string passing.
