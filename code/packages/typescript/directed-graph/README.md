# @coding-adventures/directed-graph

A directed graph library with topological sort, cycle detection, transitive closure, and parallel execution level computation. Built for use in build systems, dependency resolution, and task scheduling.

## Where it fits in the stack

This package provides the graph data structure that underpins dependency resolution across the coding-adventures project. Any package that needs to reason about "what depends on what" -- build ordering, incremental rebuilds, parallel execution planning -- uses this library.

This is the TypeScript port of the Python `coding-adventures-directed-graph` package.

## Installation

```bash
npm install @coding-adventures/directed-graph
```

For development:

```bash
npm install
```

## Quick Start

```typescript
import { Graph } from "@coding-adventures/directed-graph";

// Build a dependency graph
const g = new Graph();
g.addEdge("compile", "parse");     // compile depends on parse
g.addEdge("compile", "typecheck"); // compile depends on typecheck
g.addEdge("link", "compile");      // link depends on compile
g.addEdge("package", "link");      // package depends on link

// Get a valid build order
console.log(g.topologicalSort());
// ['parse', 'typecheck', 'compile', 'link', 'package']

// Find what can run in parallel
console.log(g.independentGroups());
// [['parse', 'typecheck'], ['compile'], ['link'], ['package']]

// What's affected if we change 'parse'?
console.log(g.affectedNodes(new Set(["parse"])));
// Set { 'parse', 'compile', 'link', 'package' }
```

## API Reference

### Graph

#### Core Methods

| Method | Description |
|--------|-------------|
| `addNode(node)` | Add a node (no-op if exists) |
| `addEdge(from, to)` | Add directed edge; implicitly adds nodes; throws `Error` on self-loop |
| `removeNode(node)` | Remove node and all edges; throws `NodeNotFoundError` |
| `removeEdge(from, to)` | Remove edge; throws `EdgeNotFoundError` |
| `hasNode(node)` | Check if node exists |
| `hasEdge(from, to)` | Check if edge exists |
| `nodes()` | List all nodes |
| `edges()` | List all edges as `[from, to]` tuples |
| `predecessors(node)` | Direct parents of node |
| `successors(node)` | Direct children of node |
| `size` | Number of nodes (getter) |

#### Algorithm Methods

| Method | Description |
|--------|-------------|
| `topologicalSort()` | Kahn's algorithm; throws `CycleError` |
| `hasCycle()` | DFS three-color cycle detection |
| `transitiveClosure(node)` | All nodes reachable downstream (returns `Set`) |
| `transitiveDependents(node)` | All nodes that depend on node, reverse (returns `Set`) |
| `independentGroups()` | Partition into parallel execution levels |
| `affectedNodes(changed)` | Changed nodes + all transitive dependents (returns `Set`) |

### Error Classes

- `CycleError` -- includes `.cycle` property with the cycle path
- `NodeNotFoundError` -- includes `.node` property
- `EdgeNotFoundError` -- includes `.fromNode` and `.toNode` properties

## Internal Design

The graph uses two adjacency maps (`Map<string, Set<string>>`):

- `_forward.get(u)` = set of nodes that u points to (successors)
- `_reverse.get(v)` = set of nodes that point to v (predecessors)

This dual-map design makes both forward and reverse traversals O(1) per edge, which is essential for algorithms like `transitiveDependents` that walk edges backwards.

## Running Tests

```bash
npx vitest run --coverage
```

Tests require 80%+ line coverage (enforced by vitest configuration).
