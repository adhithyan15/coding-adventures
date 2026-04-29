# coding_adventures_graph

Undirected weighted graph package for Dart with interchangeable adjacency-list
and adjacency-matrix storage.

## What It Provides

- `Graph` with `GraphRepr.adjacencyList` and `GraphRepr.adjacencyMatrix`
- Weighted undirected edges, including self-loops
- Graph, node, and edge property bags for metadata and future graph runtimes
- `bfs`, `dfs`, `isConnected`, `connectedComponents`, and `hasCycle`
- `shortestPath` and `minimumSpanningTree`

## Usage

```dart
import 'package:coding_adventures_graph/graph.dart';

void main() {
  final graph = Graph<String>(GraphRepr.adjacencyList);
  graph.addEdge('London', 'Paris', 300);
  graph.addEdge('London', 'Amsterdam', 520);
  graph.addEdge('Amsterdam', 'Berlin', 655);
  graph.setNodeProperty('London', 'kind', 'city');

  print(shortestPath(graph, 'London', 'Berlin'));
  // [London, Amsterdam, Berlin]
}
```

## Building and Testing

```bash
dart pub get
dart test
```
