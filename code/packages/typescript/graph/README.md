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

## Example

```ts
import {
  Graph,
  GraphRepr,
  shortestPath,
} from "@coding-adventures/graph";

const graph = new Graph<string>(GraphRepr.ADJACENCY_LIST);
graph.addEdge("London", "Paris", 300);
graph.addEdge("Paris", "Berlin", 878);
graph.addEdge("London", "Amsterdam", 520);
graph.addEdge("Amsterdam", "Berlin", 655);

console.log(shortestPath(graph, "London", "Berlin"));
// ["London", "Amsterdam", "Berlin"]
```

## Notes

- Edges are undirected, so adding `A`-`B` also creates `B`-`A`.
- Nodes are generic and may be strings, numbers, tuples, or object references.
- The graph stores isolated nodes as well as connected ones.
