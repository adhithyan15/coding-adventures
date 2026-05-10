# coding-adventures-multi-directed-graph

Generic Python multi-directed graph with stable edge IDs, parallel directed
edges, graph/node/edge property bags, edge weights, topological sorting, cycle
detection, and independent execution groups.

This package is domain-neutral. Neural-network concepts are layered on top by
`coding-adventures-neural-network`.

```python
from multi_directed_graph import MultiDirectedGraph

graph = MultiDirectedGraph[str]()
graph.add_node("x", {"kind": "input"})
edge_id = graph.add_edge("x", "sum", 0.25, {"trainable": True}, "w0")

assert graph.edge_properties(edge_id) == {"trainable": True, "weight": 0.25}
assert graph.topological_sort() == ["x", "sum"]
```
