import 'package:coding_adventures_directed_graph/directed_graph.dart';
import 'package:test/test.dart';

void main() {
  group('Graph', () {
    test('supports nodes and edges', () {
      final graph = Graph();
      graph.addEdge('A', 'B');

      expect(graph.hasNode('A'), isTrue);
      expect(graph.hasNode('B'), isTrue);
      expect(graph.hasEdge('A', 'B'), isTrue);
      expect(graph.predecessors('B'), ['A']);
      expect(graph.successors('A'), ['B']);
    });

    test('topological sort orders dependencies', () {
      final graph = Graph();
      graph.addEdge('compile', 'parse');
      graph.addEdge('compile', 'typecheck');
      graph.addEdge('link', 'compile');

      expect(graph.topologicalSort(), ['link', 'compile', 'parse', 'typecheck']);
    });

    test('detects cycles', () {
      final graph = Graph();
      graph.addEdge('A', 'B');
      graph.addEdge('B', 'C');
      graph.addEdge('C', 'A');

      expect(graph.hasCycle(), isTrue);
      expect(() => graph.topologicalSort(), throwsA(isA<CycleError>()));
    });

    test('computes transitive closure and dependents', () {
      final graph = Graph();
      graph.addEdge('A', 'B');
      graph.addEdge('B', 'C');
      graph.addEdge('D', 'B');

      expect(graph.transitiveClosure('A'), {'B', 'C'});
      expect(graph.transitiveDependents('C'), {'A', 'B', 'D'});
    });

    test('computes independent groups', () {
      final graph = Graph();
      graph.addEdge('A', 'B');
      graph.addEdge('X', 'Y');

      expect(graph.independentGroups(), [
        ['A', 'X'],
        ['B', 'Y'],
      ]);
    });

    test('computes affected nodes', () {
      final graph = Graph();
      graph.addEdge('A', 'B');
      graph.addEdge('B', 'C');

      expect(graph.affectedNodes({'C'}), {'A', 'B', 'C'});
      expect(graph.affectedNodes({'Z'}), isEmpty);
    });

    test('supports self-loops when enabled', () {
      final graph = Graph(allowSelfLoops: true);
      graph.addEdge('A', 'A');

      expect(graph.hasEdge('A', 'A'), isTrue);
      expect(graph.successors('A'), ['A']);
      expect(graph.predecessors('A'), ['A']);
    });
  });

  group('LabeledDirectedGraph', () {
    test('tracks multiple labels on one edge', () {
      final graph = LabeledDirectedGraph();
      graph.addEdge('locked', 'unlocked', 'coin');
      graph.addEdge('locked', 'unlocked', 'token');

      expect(graph.hasEdge('locked', 'unlocked'), isTrue);
      expect(graph.hasEdge('locked', 'unlocked', 'coin'), isTrue);
      expect(graph.labels('locked', 'unlocked'), {'coin', 'token'});
    });

    test('filters neighbors by label', () {
      final graph = LabeledDirectedGraph();
      graph.addEdge('locked', 'unlocked', 'coin');
      graph.addEdge('locked', 'locked', 'push');

      expect(graph.successors('locked', 'coin'), ['unlocked']);
      expect(graph.successors('locked', 'push'), ['locked']);
      expect(graph.predecessors('locked', 'push'), ['locked']);
    });
  });

  group('Visualization', () {
    test('renders plain graph formats', () {
      final graph = Graph();
      graph.addEdge('A', 'B');

      expect(toDot(graph), contains('A -> B'));
      expect(toMermaid(graph), contains('A --> B'));
      expect(toAsciiTable(graph), contains('A | B'));
    });

    test('renders labeled graph formats', () {
      final graph = LabeledDirectedGraph();
      graph.addEdge('locked', 'unlocked', 'coin');

      expect(toDot(graph), contains('[label="coin"]'));
      expect(toMermaid(graph), contains('-->|coin|'));
      expect(toAsciiTable(graph), contains('locked | unlocked | coin'));
    });
  });
}
