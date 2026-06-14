# @coding-adventures/multi-directed-graph

A generic multi-directed graph with stable edge IDs, parallel directed edges,
weights, and graph/node/edge property bags. Node values can be strings, numbers,
or any value that can be used as a JavaScript `Map` key.

This package is domain-neutral. Neural-network packages can build primitives on
top of it, but the graph itself does not know about activation functions, tensor
shapes, or training. Unlike a simple directed graph, multiple edges can connect
the same ordered pair of nodes:

```typescript
import { MultiDirectedGraph } from "@coding-adventures/multi-directed-graph";

const graph = new MultiDirectedGraph();
const e0 = graph.addEdge("x0", "sum", 0.25, { "nn.trainable": true });
const e1 = graph.addEdge("x0", "sum", 0.75, { "nn.channel": "skip" });

graph.edgeWeight(e0); // 0.25
graph.edgeProperties(e1); // { "nn.channel": "skip", weight: 0.75 }
```

## API

| Method | Description |
| --- | --- |
| `addNode(node, properties)` | Add a node or merge node metadata. |
| `removeNode(node)` | Remove a node and all incident edges. |
| `nodes()` | Return node IDs in insertion order. |
| `addEdge(from, to, weight, properties, edgeId)` | Add a directed edge and return its stable edge ID. |
| `removeEdge(edgeId)` | Remove one directed edge by ID. |
| `edges()` | Return all edge records. |
| `edgesBetween(from, to)` | Return parallel edges from `from` to `to`. |
| `outgoingEdges(node)` | Return outgoing edge records. |
| `incomingEdges(node)` | Return incoming edge records. |
| `successors(node)` | Return unique successor node IDs. |
| `predecessors(node)` | Return unique predecessor node IDs. |
| `edgeWeight(edgeId)` | Return one edge weight. |
| `topologicalSort()` | Return a topological node order or throw on cycles. |
| `independentGroups()` | Return parallel execution levels or throw on cycles. |

Property methods mirror DT00/DT01:

```typescript
graph.setGraphProperty("nn.name", "tiny-model");
graph.setNodeProperty("sum", "nn.op", "weighted_sum");
graph.setEdgeProperty(e0, "weight", 0.5);
```

`weight` is canonical edge metadata. Setting the `weight` property updates
`edgeWeight(edgeId)`, and removing it resets the edge weight to `1.0`.
