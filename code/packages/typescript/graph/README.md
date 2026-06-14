# @coding-adventures/graph

An undirected weighted graph for TypeScript with two interchangeable internal
representations:

- `GraphRepr.ADJACENCY_LIST` for sparse graphs
- `GraphRepr.ADJACENCY_MATRIX` for dense graphs

The package exposes one generic `Graph<T>` class plus pure algorithm helpers:

- `bfs`
- `dfs`
- `isConnected`
- `connectedComponents`
- `hasCycle`
- `shortestPath`
- `minimumSpanningTree`

Graphs also support portable graph, node, and edge property bags. Edge weights are
available as the canonical `weight` edge property, so higher-level packages can treat
weighted graphs, labeled graphs, visualizers, and neural graph builders as the same
topology-plus-metadata model.

## Example

```ts
import {
  Graph,
  GraphRepr,
  shortestPath,
} from "@coding-adventures/graph";

const graph = new Graph<string>(GraphRepr.ADJACENCY_LIST);
graph.addNode("London", { kind: "city" });
graph.addEdge("London", "Paris", 300, { route: "train" });
graph.addEdge("Paris", "Berlin", 878);
graph.addEdge("London", "Amsterdam", 520);
graph.addEdge("Amsterdam", "Berlin", 655);

console.log(graph.edgeProperties("Paris", "London"));
// { route: "train", weight: 300 }

console.log(shortestPath(graph, "London", "Berlin"));
// ["London", "Amsterdam", "Berlin"]
```

## Notes

- Edges are undirected, so adding `A`-`B` also creates `B`-`A`.
- Nodes are generic and may be strings, numbers, tuples, or object references.
- The graph stores isolated nodes as well as connected ones.
- Property bags are copied on read so callers cannot accidentally mutate the graph.
