# graph-wasm

WebAssembly bindings for the Rust [graph](../../rust/graph/) crate.

## What It Provides

- `WasmGraph` with adjacency-list and adjacency-matrix backing
- String-node graph operations for browser and JS runtimes
- `bfs`, `dfs`, `isConnected`, `connectedComponentsJson`, `hasCycle`
- `shortestPath` and `minimumSpanningTreeJson`

## Usage

```javascript
import init, { WasmGraph } from "./graph_wasm.js";

await init();

const graph = WasmGraph.withRepresentation("adjacency_matrix");
graph.addEdge("London", "Paris", 300);
graph.addEdge("London", "Amsterdam", 520);
graph.addEdge("Amsterdam", "Berlin", 655);

graph.shortestPath("London", "Berlin"); // ["London", "Amsterdam", "Berlin"]
graph.minimumSpanningTreeJson(); // JSON string of weighted edges
```

## Building

```bash
cargo test
wasm-pack build --target web
```
