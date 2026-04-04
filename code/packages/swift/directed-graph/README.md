# DirectedGraph

A directed graph data structure with algorithms for topological sorting, cycle detection, transitive closure, and parallel execution level computation.

## Overview

This package implements a directed graph using dual adjacency maps (forward + reverse) for O(1) neighbor lookups in both directions. It is a foundational data structure used by the grammar-tools and build system packages.

## Usage

```swift
import DirectedGraph

var graph = Graph()
try graph.addEdge(from: "A", to: "B")
try graph.addEdge(from: "B", to: "C")

let sorted = try graph.topologicalSort()  // ["A", "B", "C"]
let closure = try graph.transitiveClosure(of: "A")  // {"B", "C"}
let groups = try graph.independentGroups()  // [["A"], ["B"], ["C"]]
```

## API

- `addNode(_:)` / `removeNode(_:)` / `hasNode(_:)` / `nodes()`
- `addEdge(from:to:)` / `removeEdge(from:to:)` / `hasEdge(from:to:)` / `edges()`
- `successors(of:)` / `predecessors(of:)`
- `topologicalSort()` / `hasCycle()`
- `transitiveClosure(of:)` / `transitiveDependents(of:)`
- `independentGroups()` / `affectedNodes(changed:)`
