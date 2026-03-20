# Tree (Go)

A rooted tree data structure backed by a directed graph, with traversals, lowest common ancestor, subtree extraction, and ASCII visualization.

## How It Fits in the Stack

This package builds on top of the `directed-graph` Go package. The directed graph handles all low-level node/edge storage, while this `Tree` type enforces tree invariants (single root, single parent per node, no cycles) and provides tree-specific operations.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/tree"

t := tree.New("Program")
t.AddChild("Program", "Assignment")
t.AddChild("Program", "Print")
t.AddChild("Assignment", "Name")
t.AddChild("Assignment", "BinaryOp")

fmt.Println(t.ToAscii())
// Program
// +-- Assignment
// |   +-- BinaryOp
// |   +-- Name
// +-- Print
```

## API

- `New(root)` -- create a tree with the given root
- `AddChild(parent, child)` -- add a child under parent
- `RemoveSubtree(node)` -- remove a node and all descendants
- `Root()`, `Parent(node)`, `Children(node)`, `Siblings(node)`
- `IsLeaf(node)`, `IsRoot(node)`, `Depth(node)`, `Height()`, `Size()`
- `Nodes()`, `Leaves()`, `HasNode(node)`
- `Preorder()`, `Postorder()`, `LevelOrder()`
- `PathTo(node)`, `LCA(a, b)`, `Subtree(node)`
- `ToAscii()` -- ASCII visualization
- `Graph()` -- access the underlying directed graph
