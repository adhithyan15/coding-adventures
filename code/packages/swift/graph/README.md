# Graph (Swift) — DT00

A generic, undirected weighted graph data structure implemented in pure Swift from scratch.

## Overview

This package provides a complete, generic undirected graph implementation with:

- **Generic over any Hashable node type**: Works with String, Int, custom types, etc.
- **Two representations**: adjacency list (default, O(V+E) space) or adjacency matrix (O(V²) space, O(1) edge lookup)
- **Core operations**: node/edge management, degree, neighbourhood queries
- **Graph algorithms**: BFS, DFS, shortest path (BFS or Dijkstra), cycle detection, connected components, minimum spanning tree, bipartite checking
- **Weighted edges**: default weight 1.0, customizable per edge
- **Value semantics**: Graph is a struct (value type), not a class
- **Comprehensive error handling**: GraphError enum for proper error propagation

## Installation

Add this to your `Package.swift`:

```swift
.package(path: "../graph")
```

## Quick Start

```swift
import Graph

// Create a graph with adjacency list (default)
var g: Graph<String> = Graph()
// or with adjacency matrix for dense graphs:
var g = Graph<String>(repr: .adjacencyMatrix)

// Add edges (creates nodes automatically)
g.addEdge("A", "B", weight: 1.5)
g.addEdge("B", "C", weight: 2.0)

// Node queries
g.hasNode("A")           // true
g.nodes                  // all nodes
try g.degree("A")        // number of neighbours (throws if node not found)

// Edge queries
g.hasEdge("A", "B")      // true
try g.edgeWeight("A", "B") // 1.5 (throws if edge not found)
try g.neighbors("A")     // ["B"] (throws if node not found)

// Traversals
let bfs = g.bfs("A")
let dfs = g.dfs("A")

// Algorithms
let connected = g.isConnected
let hasCycle = g.hasCycle()
let path = g.shortestPath("A", "C")
let mst = try g.minimumSpanningTree()
let bipartite = g.isBipartite()
```

## API

### Types

- `GraphRepr` — Enum: `.adjacencyList` or `.adjacencyMatrix`
- `GraphError` — Error type with cases: `.nodeNotFound`, `.edgeNotFound`, `.graphNotConnected`, `.invalidRepresentation`
- `Graph<Node: Hashable & CustomStringConvertible>` — Generic struct

### Construction

- `init(repr: GraphRepr = .adjacencyList)` — Create graph

### Node Operations

- `addNode(_:)` — Add node (no-op if exists)
- `removeNode(_:)` throws — Remove node and incident edges
- `hasNode(_:)` — Check existence
- `var nodes: [Node]` — All nodes
- `var count: Int` — Node count

### Edge Operations

- `addEdge(_:_:weight:)` — Add edge with weight (default 1.0)
- `removeEdge(_:_:)` throws — Remove edge
- `hasEdge(_:_:)` — Check existence
- `var edges: [(Node, Node, Double)]` — All edges
- `edgeWeight(_:_:)` throws -> Double — Get weight

### Neighbourhood Queries

- `neighbors(_:)` throws -> [Node] — Neighbours
- `neighborsWeighted(_:)` throws -> [Node: Double] — Weighted neighbours
- `degree(_:)` throws -> Int — Number of incident edges

### Computed Properties

- `var isConnected: Bool` — True if all nodes reachable

### Algorithms

- `bfs(_:)` -> [Node] — Breadth-first search. Time: O(V+E)
- `dfs(_:)` -> [Node] — Depth-first search. Time: O(V+E)
- `hasCycle()` -> Bool — True if contains cycle. Time: O(V+E)
- `isBipartite()` -> Bool — True if 2-colorable. Time: O(V+E)
- `shortestPath(_:_:)` -> [Node] — Shortest path. Time: O(V+E) or O((V+E) log V)
- `minimumSpanningTree()` throws -> [(Node, Node, Double)] — MST. Time: O(E log E)

## Design Patterns

### Generic Over Hashable

Works with any hashable type that conforms to CustomStringConvertible:

```swift
var g1: Graph<String> = Graph()
var g2: Graph<Int> = Graph()
var g3: Graph<UUID> = Graph()  // if UUID conforms to CustomStringConvertible
```

### Throwing vs Non-Throwing

- **Throwing methods**: Operations that query a specific node (neighbors, degree, etc.)
- **Non-throwing methods**: Global operations (BFS, isConnected, etc.)
- **Property access**: Use properties (isConnected, count) for common queries

### Representation Agnosticism

Algorithms work identically on both representations:

```swift
var gList = Graph<String>(repr: .adjacencyList)
var gMatrix = Graph<String>(repr: .adjacencyMatrix)

gList.addEdge("A", "B")
gMatrix.addEdge("A", "B")

let bfs1 = gList.bfs("A")
let bfs2 = gMatrix.bfs("A")  // Same result, different internal structure
```

## Theory

A graph G = (V, E) consists of:
- **V**: vertices (nodes) — any Hashable type
- **E**: edges — unordered pairs {u, v} with optional weights

Undirected means {u,v} = {v,u}. This implementation maintains full symmetry automatically.

## Testing

```bash
swift test
```

Comprehensive test suite with 41 test cases covering:
- Both representations (adjacency list and matrix)
- Node/edge operations
- All algorithms with various graph topologies
- Edge cases (empty, single-node, disconnected graphs)
- Error conditions
- Generic type parameters

## Performance

**Adjacency List** (default):
- Space: O(V + E)
- Edge lookup: O(degree(u))
- Best for sparse graphs

**Adjacency Matrix**:
- Space: O(V²)
- Edge lookup: O(1)
- Best for dense graphs or when O(1) edge lookup is critical

## License

MIT
