# DT02: Multi-Directed Graph

Status: draft

DT02 extends DT01 directed graphs with stable edge identity and parallel edges.
It remains a generic data structure: no neural-network, compiler, visualization,
or runtime concepts are built into the graph itself.

## Model

```text
G = (V, E)

V = set of nodes
E = set of directed edge records

Edge = {
  id: EdgeId,
  from: Node,
  to: Node,
  weight: number,
  properties: PropertyBag
}
```

Unlike DT01, `(from, to)` is not unique. Multiple edges may connect the same
ordered pair:

```text
e0: A -> B, weight=0.25
e1: A -> B, weight=0.75
```

Those edges are independent. Removing or updating `e0` must not affect `e1`.

## Generic Node Values

Implementations should support the same node-value genericity as the host graph
package whenever the language can represent it:

```text
TypeScript: MultiDirectedGraph<T = string>
Python:     MultiDirectedGraph[T]
Rust:       MultiDirectedGraph<T: Eq + Hash + Clone>
Go:         MultiDirectedGraph[T comparable]
```

The reference TypeScript package uses strings for edge IDs but generic node
values.

## Property Bags

DT02 inherits the DT00 property model:

```text
graph_properties: PropertyBag
node_properties[node]: PropertyBag
edge_properties[edge_id]: PropertyBag
```

`weight` is the canonical edge property. Setting `weight` through edge metadata
must update the edge's weight API. Removing `weight` resets the edge weight to
`1.0`.

## Required Operations

```text
add_node(node, properties = {})
remove_node(node)
has_node(node) -> bool
nodes() -> Node[]

add_edge(from, to, weight = 1.0, properties = {}, edge_id = auto) -> EdgeId
remove_edge(edge_id)
has_edge(edge_id) -> bool
edge(edge_id) -> Edge
edges() -> Edge[]
edges_between(from, to) -> Edge[]
incoming_edges(node) -> Edge[]
outgoing_edges(node) -> Edge[]
successors(node) -> Node[]
predecessors(node) -> Node[]
edge_weight(edge_id) -> number

graph_properties()
set_graph_property(key, value)
remove_graph_property(key)
node_properties(node)
set_node_property(node, key, value)
remove_node_property(node, key)
edge_properties(edge_id)
set_edge_property(edge_id, key, value)
remove_edge_property(edge_id, key)
```

## Algorithms

Topological algorithms must account for edge multiplicity. If two parallel
edges enter the same node, that node's in-degree includes both edges. When the
predecessor is processed, both edge contributions are removed.

Required v0 algorithms:

```text
topological_sort() -> Node[]
has_cycle() -> bool
independent_groups() -> Node[][]
```

Parallel edges alone do not create cycles. Self-loops are cycles when allowed.

## Error Rules

Implementations should raise language-idiomatic errors for:

- Missing node.
- Missing edge ID.
- Duplicate explicit edge ID.
- Non-numeric weight.
- Self-loop when self-loops are disabled.
- Cycle during topological sort or independent-group calculation.

## Serialization Notes

DT02 should serialize edges by ID:

```json
{
  "nodes": ["A", "B"],
  "edges": [
    { "id": "e0", "from": "A", "to": "B", "weight": 0.25 },
    { "id": "e1", "from": "A", "to": "B", "weight": 0.75 }
  ],
  "edge_properties": {
    "e0": { "weight": 0.25, "channel": "left" },
    "e1": { "weight": 0.75, "channel": "right" }
  }
}
```

Node values that are not naturally serializable need language-specific encoding
or an application-provided node ID mapping.
