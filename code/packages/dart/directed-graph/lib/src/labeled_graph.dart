import 'errors.dart';
import 'graph.dart';

typedef LabeledEdge = ({String from, String to, String label});

class LabeledDirectedGraph {
  LabeledDirectedGraph({bool allowSelfLoops = true})
      : _graph = Graph(allowSelfLoops: allowSelfLoops);

  final Graph _graph;
  final Map<String, Set<String>> _labels = <String, Set<String>>{};

  Graph get graph => _graph;

  int get size => _graph.size;

  void addNode(String node) => _graph.addNode(node);

  void removeNode(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFoundError(node);
    }

    final toDelete = <String>[];
    for (final key in _labels.keys) {
      final parts = key.split('\u0000');
      if (parts[0] == node || parts[1] == node) {
        toDelete.add(key);
      }
    }
    for (final key in toDelete) {
      _labels.remove(key);
    }

    _graph.removeNode(node);
  }

  bool hasNode(String node) => _graph.hasNode(node);

  List<String> nodes() => _graph.nodes();

  void addEdge(String fromNode, String toNode, String label) {
    if (!_graph.hasEdge(fromNode, toNode)) {
      _graph.addEdge(fromNode, toNode);
    }
    _labels.putIfAbsent(_edgeKey(fromNode, toNode), () => <String>{}).add(label);
  }

  void removeEdge(String fromNode, String toNode, String label) {
    if (!_graph.hasNode(fromNode)) {
      throw NodeNotFoundError(fromNode);
    }
    if (!_graph.hasNode(toNode)) {
      throw NodeNotFoundError(toNode);
    }

    final key = _edgeKey(fromNode, toNode);
    final labels = _labels[key];
    if (labels == null || !labels.contains(label)) {
      throw EdgeNotFoundError(fromNode, toNode);
    }

    labels.remove(label);
    if (labels.isEmpty) {
      _labels.remove(key);
      _graph.removeEdge(fromNode, toNode);
    }
  }

  bool hasEdge(String fromNode, String toNode, [String? label]) {
    if (label == null) {
      return _graph.hasEdge(fromNode, toNode);
    }
    return _labels[_edgeKey(fromNode, toNode)]?.contains(label) ?? false;
  }

  List<LabeledEdge> edges() {
    final result = <LabeledEdge>[];
    final keys = _labels.keys.toList()..sort();
    for (final key in keys) {
      final parts = key.split('\u0000');
      final labels = _labels[key]!.toList()..sort();
      for (final label in labels) {
        result.add((from: parts[0], to: parts[1], label: label));
      }
    }
    return List<LabeledEdge>.unmodifiable(result);
  }

  Set<String> labels(String fromNode, String toNode) {
    if (!_graph.hasNode(fromNode)) {
      throw NodeNotFoundError(fromNode);
    }
    if (!_graph.hasNode(toNode)) {
      throw NodeNotFoundError(toNode);
    }
    return Set<String>.unmodifiable(
      _labels[_edgeKey(fromNode, toNode)] ?? <String>{},
    );
  }

  List<String> successors(String node, [String? label]) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFoundError(node);
    }
    if (label == null) {
      return _graph.successors(node);
    }

    final result = <String>[];
    for (final successor in _graph.successors(node)) {
      if (_labels[_edgeKey(node, successor)]?.contains(label) ?? false) {
        result.add(successor);
      }
    }
    result.sort();
    return List<String>.unmodifiable(result);
  }

  List<String> predecessors(String node, [String? label]) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFoundError(node);
    }
    if (label == null) {
      return _graph.predecessors(node);
    }

    final result = <String>[];
    for (final predecessor in _graph.predecessors(node)) {
      if (_labels[_edgeKey(predecessor, node)]?.contains(label) ?? false) {
        result.add(predecessor);
      }
    }
    result.sort();
    return List<String>.unmodifiable(result);
  }

  List<String> topologicalSort() => _graph.topologicalSort();

  bool hasCycle() => _graph.hasCycle();

  Set<String> transitiveClosure(String node) => _graph.transitiveClosure(node);

  Set<String> transitiveDependents(String node) =>
      _graph.transitiveDependents(node);

  List<List<String>> independentGroups() => _graph.independentGroups();

  Set<String> affectedNodes(Set<String> changed) => _graph.affectedNodes(changed);
}

String _edgeKey(String fromNode, String toNode) => '$fromNode\u0000$toNode';
