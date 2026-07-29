# Multi Directed Graph

A generic Go multi-directed graph implementing
[DT02](../../../specs/DT02-multi-directed-graph.md). It keeps insertion-ordered
nodes, stable edge IDs, parallel directed edges, optional self-loops, numeric
edge weights, and metadata property bags on the graph, nodes, and edges.

The package is domain-neutral. Higher-level packages such as `neural-network`
layer their semantics on top instead of baking them into the graph.

```go
package main

import (
	"fmt"

	multidirectedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/multi-directed-graph"
)

func main() {
	graph := multidirectedgraph.New[string]()
	left, _ := graph.AddEdge(
		"x",
		"sum",
		0.25,
		multidirectedgraph.PropertyBag{"channel": "left"},
		"w0",
	)
	right, _ := graph.AddEdge("x", "sum", 0.75, nil, "w1")

	fmt.Println(left, right) // w0 w1
	fmt.Println(graph.EdgesBetween("x", "sum"))
}
```

## API

| Operation | Description |
| --- | --- |
| `AddNode`, `RemoveNode`, `HasNode`, `Nodes` | Manage generic comparable node values. |
| `AddEdge`, `RemoveEdge`, `HasEdge`, `Edge`, `Edges` | Manage directed edges by stable ID. |
| `EdgesBetween` | Return every parallel edge for one ordered node pair. |
| `OutgoingEdges`, `IncomingEdges` | Return incident edge records in insertion order. |
| `Successors`, `Predecessors` | Return unique neighboring nodes. |
| `EdgeWeight` | Read the canonical numeric edge weight. |
| `GraphProperties`, `NodeProperties`, `EdgeProperties` | Read copied metadata bags. |
| `Set*Property`, `Remove*Property` | Mutate graph, node, or edge metadata. |
| `TopologicalSort`, `HasCycle`, `IndependentGroups` | Run multiplicity-aware DAG algorithms. |

Missing nodes and edge IDs return `NodeNotFoundError` and
`EdgeNotFoundError`. Duplicate explicit IDs return `DuplicateEdgeIDError`.
Setting the `weight` edge property updates `EdgeWeight`; removing it resets the
weight to `1.0`.
