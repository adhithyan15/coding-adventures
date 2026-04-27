# @coding-adventures/directed-graph-native

A Rust-backed directed graph library for Node.js via napi-rs. Drop-in native replacement for `@coding-adventures/directed-graph` with the same API but native performance.

## What is this?

This package wraps the Rust `directed-graph` crate and exposes it to Node.js as a native addon. All algorithms (topological sort, cycle detection, transitive closure, independent groups, affected nodes) run in Rust -- only the method call boundary crosses between JavaScript and Rust via N-API.

This is the Node.js counterpart to the Python native extension at `code/packages/python/directed-graph-native/`. Both wrap the same Rust crate and provide identical behavior.

## Installation

```bash
npm install @coding-adventures/directed-graph-native
```

Requires a Rust toolchain for building from source.

## Usage

```typescript
import { DirectedGraph } from '@coding-adventures/directed-graph-native';

const g = new DirectedGraph();
g.addEdge("compile", "link");
g.addEdge("link", "package");

// Topological sort (Kahn's algorithm)
console.log(g.topologicalSort());    // ['compile', 'link', 'package']

// Parallel execution levels
console.log(g.independentGroups());  // [['compile'], ['link'], ['package']]

// Cycle detection
console.log(g.hasCycle());           // false

// Incremental builds: what's affected by a change?
const affected = g.affectedNodes(["compile"]);
console.log(affected);               // ['compile', 'link', 'package']
```

## API

| Method | Description |
|--------|-------------|
| `addNode(name)` | Add a node |
| `removeNode(name)` | Remove a node and its edges |
| `hasNode(name)` | Check if node exists |
| `nodes()` | Sorted list of all nodes |
| `addEdge(from, to)` | Add directed edge |
| `removeEdge(from, to)` | Remove edge |
| `hasEdge(from, to)` | Check if edge exists |
| `edges()` | Sorted list of `[from, to]` pairs |
| `predecessors(node)` | Nodes pointing to this node |
| `successors(node)` | Nodes this node points to |
| `size()` | Number of nodes |
| `edgeCount()` | Number of edges |
| `toStringRepr()` | Human-readable description |
| `topologicalSort()` | Kahn's algorithm |
| `hasCycle()` | DFS 3-color cycle detection |
| `transitiveClosure(node)` | All reachable nodes |
| `affectedNodes(changed)` | Changed + transitive dependents |
| `independentGroups()` | Parallel execution levels |

## Error handling

Errors are thrown as plain JavaScript `Error` objects with descriptive message prefixes:

- `"CycleError: ..."` -- topological sort or independent groups on a cyclic graph
- `"NodeNotFoundError: ..."` -- operation on a nonexistent node
- `"EdgeNotFoundError: ..."` -- removing a nonexistent edge
- `"SelfLoopError: ..."` -- adding an edge from a node to itself

## How it fits in the stack

This is the Node.js counterpart to the Python native extension. Both wrap the same Rust `directed-graph` crate, demonstrating how a single Rust library can serve multiple language ecosystems:

- **Rust core**: `code/packages/rust/directed-graph/` -- all algorithms
- **Python wrapper**: `code/packages/python/directed-graph-native/` -- PyO3
- **Node.js wrapper**: `code/packages/typescript/directed-graph-native/` -- napi-rs
- **Pure TypeScript**: `code/packages/typescript/directed-graph/` -- educational reimplementation

## Development

```bash
# Install dependencies
npm install

# Build the native addon
npx napi build --release --platform

# Run tests
npx vitest run
```

Requires Rust toolchain (`rustup`) and Node.js 16+.
