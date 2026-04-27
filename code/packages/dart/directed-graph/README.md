# coding_adventures_directed_graph

Directed graph package for Dart with topological sort, cycle detection,
transitive closure, parallel execution levels, labeled edges, and text
visualization helpers.

## Usage

```dart
import 'package:coding_adventures_directed_graph/directed_graph.dart';

void main() {
  final graph = Graph();
  graph.addEdge('compile', 'parse');
  graph.addEdge('link', 'compile');

  print(graph.topologicalSort());
  // [link, compile, parse]
}
```

## Building and Testing

```bash
dart pub get
dart test
```
