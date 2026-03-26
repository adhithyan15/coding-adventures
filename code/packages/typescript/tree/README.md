# Tree (TypeScript)

A rooted tree data structure backed by a directed graph, with traversals, lowest common ancestor, subtree extraction, and ASCII visualization.

## How It Fits in the Stack

This package builds on top of `@coding-adventures/directed-graph`. The directed graph handles all low-level node/edge storage, while this `Tree` class enforces tree invariants (single root, single parent per node, no cycles) and provides tree-specific operations.

## Installation

```json
{
  "dependencies": {
    "@coding-adventures/tree": "file:../tree"
  }
}
```

## Usage

```typescript
import { Tree } from "@coding-adventures/tree";

const t = new Tree("Program");
t.addChild("Program", "Assignment");
t.addChild("Program", "Print");
t.addChild("Assignment", "Name");
t.addChild("Assignment", "BinaryOp");

console.log(t.toAscii());
// Program
// +-- Assignment
// |   +-- BinaryOp
// |   +-- Name
// +-- Print

t.preorder();   // ["Program", "Assignment", "BinaryOp", "Name", "Print"]
t.postorder();  // ["BinaryOp", "Name", "Assignment", "Print", "Program"]
t.depth("Name");  // 2
t.lca("Name", "Print");  // "Program"
```

## API

- `new Tree(root)` -- create a tree with the given root
- `addChild(parent, child)` -- add a child under parent
- `removeSubtree(node)` -- remove a node and all descendants
- `root`, `parent(node)`, `children(node)`, `siblings(node)`
- `isLeaf(node)`, `isRoot(node)`, `depth(node)`, `height()`, `size()`
- `nodes()`, `leaves()`, `hasNode(node)`
- `preorder()`, `postorder()`, `levelOrder()`
- `pathTo(node)`, `lca(a, b)`, `subtree(node)`
- `toAscii()` -- ASCII visualization
- `graph` -- access the underlying Graph
