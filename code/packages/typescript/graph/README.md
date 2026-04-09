# @coding-adventures/graph

An undirected graph data structure implementation from scratch.

## Where it fits in the stack

This package provides a foundational undirected graph data structure for use across the coding-adventures project.

## Installation

```bash
npm install @coding-adventures/graph
```

## Quick Start

```typescript
import { Graph } from "@coding-adventures/graph";

const g = new Graph();
g.addEdge("A", "B");
g.addEdge("B", "C");

console.log(g.nodes());    // ["A", "B", "C"]
console.log(g.edges());    // [["A", "B"], ["B", "C"]]
```

## Running Tests

```bash
npm test
```

Tests require 80%+ coverage (enforced by vitest configuration).
