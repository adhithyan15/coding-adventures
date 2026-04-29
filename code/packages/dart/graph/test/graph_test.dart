import 'package:coding_adventures_graph/graph.dart';
import 'package:test/test.dart';

final representations = <GraphRepr>[
  GraphRepr.adjacencyList,
  GraphRepr.adjacencyMatrix,
];

Graph<String> makeGraph(GraphRepr repr) {
  final graph = Graph<String>(repr);
  graph.addEdge('London', 'Paris', 300);
  graph.addEdge('London', 'Amsterdam', 520);
  graph.addEdge('Paris', 'Berlin', 878);
  graph.addEdge('Amsterdam', 'Berlin', 655);
  graph.addEdge('Amsterdam', 'Brussels', 180);
  return graph;
}

Graph<String> makeTriangle(GraphRepr repr) {
  final graph = Graph<String>(repr);
  graph.addEdge('A', 'B');
  graph.addEdge('B', 'C');
  graph.addEdge('C', 'A');
  return graph;
}

Graph<String> makePath(GraphRepr repr) {
  final graph = Graph<String>(repr);
  graph.addEdge('A', 'B');
  graph.addEdge('B', 'C');
  return graph;
}

void main() {
  group('construction', () {
    test('defaults to adjacency list', () {
      expect(Graph<String>().repr, GraphRepr.adjacencyList);
    });

    for (final repr in representations) {
      test('tracks empty state for $repr', () {
        final graph = Graph<String>(repr);
        expect(graph.size, 0);
        expect(graph.nodes(), isEmpty);
      });
    }
  });

  group('node operations', () {
    for (final repr in representations) {
      test('adds and removes nodes for $repr', () {
        final graph = Graph<String>(repr);
        graph.addNode('A');
        graph.addNode('B');
        expect(graph.hasNode('A'), isTrue);
        expect(graph.size, 2);

        graph.removeNode('A');
        expect(graph.hasNode('A'), isFalse);
        expect(graph.hasNode('B'), isTrue);
        expect(graph.size, 1);
      });

      test('removing a missing node throws for $repr', () {
        final graph = Graph<String>(repr);
        expect(
          () => graph.removeNode('missing'),
          throwsA(isA<NodeNotFoundError<String>>()),
        );
      });
    }
  });

  group('edge operations', () {
    for (final repr in representations) {
      test('creates undirected edges for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B', 2.5);
        expect(graph.hasEdge('A', 'B'), isTrue);
        expect(graph.hasEdge('B', 'A'), isTrue);
        expect(graph.edgeWeight('A', 'B'), 2.5);
        expect(graph.edgeWeight('B', 'A'), 2.5);
      });

      test('keeps a self-loop visible as a neighbor for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'A');
        expect(graph.hasEdge('A', 'A'), isTrue);
        expect(graph.neighbors('A'), {'A'});
      });

      test('preserves explicit zero-weight edges for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B', 0);
        expect(graph.edgeWeight('A', 'B'), 0);
        expect(graph.edges().single.weight, 0);
      });

      test('removes edges without deleting nodes for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B');
        graph.removeEdge('A', 'B');
        expect(graph.hasNode('A'), isTrue);
        expect(graph.hasNode('B'), isTrue);
        expect(graph.hasEdge('A', 'B'), isFalse);
      });

      test('deduplicates edges() for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B', 1);
        graph.addEdge('B', 'C', 2);
        expect(graph.edges(), hasLength(2));
      });
    }
  });

  group('property bags', () {
    for (final repr in representations) {
      test('stores graph, node, and edge properties for $repr', () {
        final graph = Graph<String>(repr);

        graph.setGraphProperty('name', 'city-map');
        graph.setGraphProperty('version', 1);
        expect(graph.graphProperties(), {'name': 'city-map', 'version': 1});
        graph.removeGraphProperty('version');
        expect(graph.graphProperties(), {'name': 'city-map'});

        graph.addNode('A', {'kind': 'input'});
        graph.addNode('A', {'trainable': false});
        graph.setNodeProperty('A', 'slot', 0);
        expect(graph.nodeProperties('A'), {
          'kind': 'input',
          'trainable': false,
          'slot': 0,
        });
        graph.removeNodeProperty('A', 'slot');
        expect(graph.nodeProperties('A'), {
          'kind': 'input',
          'trainable': false,
        });

        graph.addEdge('A', 'B', 2.5, {'role': 'distance'});
        expect(graph.edgeProperties('B', 'A'), {
          'role': 'distance',
          'weight': 2.5,
        });
        graph.setEdgeProperty('B', 'A', 'weight', 7);
        expect(graph.edgeWeight('A', 'B'), 7);
        graph.setEdgeProperty('A', 'B', 'trainable', true);
        graph.removeEdgeProperty('A', 'B', 'role');
        expect(graph.edgeProperties('A', 'B'), {
          'weight': 7,
          'trainable': true,
        });

        graph.removeEdge('A', 'B');
        expect(
          () => graph.edgeProperties('A', 'B'),
          throwsA(isA<EdgeNotFoundError<String>>()),
        );
      });
    }
  });

  group('neighborhood queries', () {
    for (final repr in representations) {
      test('returns neighbors, weights, and degree for $repr', () {
        final graph = makeGraph(repr);
        expect(graph.neighbors('Amsterdam'), {'Berlin', 'Brussels', 'London'});
        expect(graph.degree('Amsterdam'), 3);
        expect(graph.neighborsWeighted('Amsterdam')['London'], 520);
        expect(graph.neighborsWeighted('Amsterdam')['Brussels'], 180);
      });
    }
  });

  group('traversals', () {
    for (final repr in representations) {
      test('runs BFS over reachable nodes for $repr', () {
        expect(bfs(makePath(repr), 'A'), ['A', 'B', 'C']);
      });

      test('runs DFS over reachable nodes for $repr', () {
        expect(dfs(makePath(repr), 'A'), ['A', 'B', 'C']);
      });

      test('limits traversal to reachable nodes for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B');
        graph.addNode('C');
        expect(bfs(graph, 'A').toSet(), {'A', 'B'});
        expect(dfs(graph, 'A').toSet(), {'A', 'B'});
      });
    }
  });

  group('connectivity', () {
    for (final repr in representations) {
      test('detects connected graphs for $repr', () {
        expect(isConnected(makeGraph(repr)), isTrue);
      });

      test('detects disconnected graphs for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B');
        graph.addNode('C');
        expect(isConnected(graph), isFalse);
      });

      test('finds connected components for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B');
        graph.addEdge('B', 'C');
        graph.addEdge('D', 'E');
        graph.addNode('F');

        final components = connectedComponents(graph);
        expect(components, hasLength(3));
        expect(
          components.any(
            (component) =>
                component.length == 3 && component.containsAll({'A', 'B', 'C'}),
          ),
          isTrue,
        );
        expect(
          components.any(
            (component) =>
                component.length == 2 && component.containsAll({'D', 'E'}),
          ),
          isTrue,
        );
        expect(
          components.any(
            (component) =>
                component.length == 1 && component.containsAll({'F'}),
          ),
          isTrue,
        );
      });
    }
  });

  group('cycle detection', () {
    for (final repr in representations) {
      test('finds a cycle in a triangle for $repr', () {
        expect(hasCycle(makeTriangle(repr)), isTrue);
      });

      test('reports no cycle in a path for $repr', () {
        expect(hasCycle(makePath(repr)), isFalse);
      });
    }
  });

  group('shortest path', () {
    for (final repr in representations) {
      test('finds unweighted shortest path for $repr', () {
        expect(shortestPath(makePath(repr), 'A', 'C'), ['A', 'B', 'C']);
      });

      test('prefers lower total weight for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B', 1);
        graph.addEdge('B', 'D', 10);
        graph.addEdge('A', 'C', 3);
        graph.addEdge('C', 'D', 3);
        expect(shortestPath(graph, 'A', 'D'), ['A', 'C', 'D']);
      });

      test('returns empty when no path exists for $repr', () {
        final graph = Graph<String>(repr);
        graph.addNode('A');
        graph.addNode('B');
        expect(shortestPath(graph, 'A', 'B'), isEmpty);
      });

      test('handles the city example for $repr', () {
        expect(shortestPath(makeGraph(repr), 'London', 'Berlin'), [
          'London',
          'Amsterdam',
          'Berlin',
        ]);
      });
    }
  });

  group('minimum spanning tree', () {
    for (final repr in representations) {
      test('returns V-1 edges for $repr', () {
        final graph = makeGraph(repr);
        expect(minimumSpanningTree(graph), hasLength(graph.size - 1));
      });

      test('picks the cheapest edges in a triangle for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B', 1);
        graph.addEdge('B', 'C', 2);
        graph.addEdge('C', 'A', 4);
        final total = minimumSpanningTree(
          graph,
        ).fold<double>(0, (sum, edge) => sum + edge.weight);
        expect(total, 3);
      });

      test('throws on disconnected graphs for $repr', () {
        final graph = Graph<String>(repr);
        graph.addEdge('A', 'B');
        graph.addNode('C');
        expect(
          () => minimumSpanningTree(graph),
          throwsA(isA<GraphNotConnectedError>()),
        );
      });
    }
  });

  group('edge cases', () {
    for (final repr in representations) {
      test('supports numeric nodes for $repr', () {
        final graph = Graph<int>(repr);
        graph.addEdge(1, 2);
        graph.addEdge(2, 3);
        expect(shortestPath(graph, 1, 3), [1, 2, 3]);
      });

      test('supports record nodes for $repr', () {
        final origin = (0, 0);
        final north = (0, 1);
        final east = (1, 1);
        final graph = Graph<(int, int)>(repr);
        graph.addEdge(origin, north);
        graph.addEdge(north, east);
        expect(isConnected(graph), isTrue);
      });
    }

    test('handles a 1000-node sparse path graph quickly enough', () {
      final graph = Graph<int>(GraphRepr.adjacencyList);
      for (var i = 0; i < 999; i++) {
        graph.addEdge(i, i + 1);
      }

      expect(graph.size, 1000);
      expect(isConnected(graph), isTrue);
      expect(hasCycle(graph), isFalse);
    });

    for (final repr in representations) {
      test('handles a complete K4 graph for $repr', () {
        final graph = Graph<String>(repr);
        final nodes = ['A', 'B', 'C', 'D'];
        for (var i = 0; i < nodes.length; i++) {
          for (var j = i + 1; j < nodes.length; j++) {
            graph.addEdge(nodes[i], nodes[j]);
          }
        }

        expect(graph.edges(), hasLength(6));
        expect(isConnected(graph), isTrue);
        expect(hasCycle(graph), isTrue);
        expect(minimumSpanningTree(graph), hasLength(3));
      });
    }
  });
}
