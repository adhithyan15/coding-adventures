import 'dart:collection';

import 'errors.dart';

typedef DirectedEdge = ({String from, String to});

class Graph {
  Graph({bool allowSelfLoops = false}) : _allowSelfLoops = allowSelfLoops;

  final bool _allowSelfLoops;
  final Map<String, Set<String>> _forward = <String, Set<String>>{};
  final Map<String, Set<String>> _reverse = <String, Set<String>>{};

  bool get allowSelfLoops => _allowSelfLoops;

  int get size => _forward.length;

  void addNode(String node) {
    _forward.putIfAbsent(node, () => <String>{});
    _reverse.putIfAbsent(node, () => <String>{});
  }

  void removeNode(String node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError(node);
    }

    for (final successor in List<String>.from(_forward[node]!)) {
      _reverse[successor]!.remove(node);
    }
    for (final predecessor in List<String>.from(_reverse[node]!)) {
      _forward[predecessor]!.remove(node);
    }

    _forward.remove(node);
    _reverse.remove(node);
  }

  bool hasNode(String node) => _forward.containsKey(node);

  List<String> nodes() {
    final result = _forward.keys.toList()..sort();
    return List<String>.unmodifiable(result);
  }

  void addEdge(String fromNode, String toNode) {
    if (fromNode == toNode && !_allowSelfLoops) {
      throw DirectedGraphException(
        'Self-loops are not allowed: "$fromNode" -> "$toNode"',
      );
    }

    addNode(fromNode);
    addNode(toNode);
    _forward[fromNode]!.add(toNode);
    _reverse[toNode]!.add(fromNode);
  }

  void removeEdge(String fromNode, String toNode) {
    final successors = _forward[fromNode];
    if (successors == null || !successors.contains(toNode)) {
      throw EdgeNotFoundError(fromNode, toNode);
    }

    successors.remove(toNode);
    _reverse[toNode]!.remove(fromNode);
  }

  bool hasEdge(String fromNode, String toNode) =>
      _forward[fromNode]?.contains(toNode) ?? false;

  List<DirectedEdge> edges() {
    final result = <DirectedEdge>[];
    for (final fromEntry in _forward.entries) {
      final sortedSuccessors = fromEntry.value.toList()..sort();
      for (final toNode in sortedSuccessors) {
        result.add((from: fromEntry.key, to: toNode));
      }
    }
    result.sort(_compareEdges);
    return List<DirectedEdge>.unmodifiable(result);
  }

  List<String> predecessors(String node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError(node);
    }
    final result = _reverse[node]!.toList()..sort();
    return List<String>.unmodifiable(result);
  }

  List<String> successors(String node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError(node);
    }
    final result = _forward[node]!.toList()..sort();
    return List<String>.unmodifiable(result);
  }

  List<String> topologicalSort() {
    final inDegree = <String, int>{};
    for (final node in nodes()) {
      inDegree[node] = _reverse[node]!.length;
    }

    final queue = ListQueue<String>();
    for (final node in nodes()) {
      if (inDegree[node] == 0) {
        queue.addLast(node);
      }
    }

    final result = <String>[];
    while (queue.isNotEmpty) {
      final node = queue.removeFirst();
      result.add(node);
      for (final successor in successors(node)) {
        final next = (inDegree[successor] ?? 0) - 1;
        inDegree[successor] = next;
        if (next == 0) {
          queue.addLast(successor);
        }
      }
    }

    if (result.length != size) {
      throw CycleError(_findCycle());
    }

    return List<String>.unmodifiable(result);
  }

  bool hasCycle() {
    final visited = <String, _VisitState>{};
    for (final node in nodes()) {
      if (_hasCycleDfs(node, visited)) {
        return true;
      }
    }
    return false;
  }

  Set<String> transitiveClosure(String node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError(node);
    }

    final visited = <String>{};
    final stack = ListQueue<String>()..add(node);
    while (stack.isNotEmpty) {
      final current = stack.removeLast();
      for (final successor in successors(current)) {
        if (visited.add(successor)) {
          stack.addLast(successor);
        }
      }
    }
    return Set<String>.unmodifiable(visited);
  }

  Set<String> transitiveDependents(String node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError(node);
    }

    final visited = <String>{};
    final stack = ListQueue<String>()..add(node);
    while (stack.isNotEmpty) {
      final current = stack.removeLast();
      for (final predecessor in predecessors(current)) {
        if (visited.add(predecessor)) {
          stack.addLast(predecessor);
        }
      }
    }
    return Set<String>.unmodifiable(visited);
  }

  List<List<String>> independentGroups() {
    final inDegree = <String, int>{};
    for (final node in nodes()) {
      inDegree[node] = _reverse[node]!.length;
    }

    var frontier = nodes().where((node) => inDegree[node] == 0).toList()..sort();
    final groups = <List<String>>[];
    var processed = 0;

    while (frontier.isNotEmpty) {
      groups.add(List<String>.unmodifiable(frontier));
      processed += frontier.length;

      final nextFrontier = <String>[];
      for (final node in frontier) {
        for (final successor in successors(node)) {
          final next = (inDegree[successor] ?? 0) - 1;
          inDegree[successor] = next;
          if (next == 0) {
            nextFrontier.add(successor);
          }
        }
      }

      nextFrontier.sort();
      frontier = nextFrontier;
    }

    if (processed != size) {
      throw CycleError(_findCycle());
    }

    return List<List<String>>.unmodifiable(groups);
  }

  Set<String> affectedNodes(Set<String> changed) {
    final affected = <String>{};
    for (final node in changed) {
      if (!hasNode(node)) {
        continue;
      }
      affected.add(node);
      affected.addAll(transitiveDependents(node));
    }
    return Set<String>.unmodifiable(affected);
  }

  @override
  String toString() => 'Graph(nodes=$size, edges=${edges().length})';

  bool _hasCycleDfs(String node, Map<String, _VisitState> visited) {
    final state = visited[node];
    if (state == _VisitState.visiting) {
      return true;
    }
    if (state == _VisitState.visited) {
      return false;
    }

    visited[node] = _VisitState.visiting;
    for (final successor in successors(node)) {
      if (_hasCycleDfs(successor, visited)) {
        return true;
      }
    }
    visited[node] = _VisitState.visited;
    return false;
  }

  List<String> _findCycle() {
    final visited = <String, _VisitState>{};
    final path = <String>[];
    final pathSet = <String>{};

    for (final node in nodes()) {
      final cycle = _findCycleDfs(node, visited, path, pathSet);
      if (cycle != null) {
        return cycle;
      }
    }

    return const <String>[];
  }

  List<String>? _findCycleDfs(
    String node,
    Map<String, _VisitState> visited,
    List<String> path,
    Set<String> pathSet,
  ) {
    final state = visited[node];
    if (state == _VisitState.visited) {
      return null;
    }
    if (state == _VisitState.visiting) {
      final start = path.indexOf(node);
      if (start >= 0) {
        return List<String>.unmodifiable(<String>[
          ...path.sublist(start),
          node,
        ]);
      }
      return List<String>.unmodifiable(<String>[node, node]);
    }

    visited[node] = _VisitState.visiting;
    path.add(node);
    pathSet.add(node);

    for (final successor in successors(node)) {
      final cycle = _findCycleDfs(successor, visited, path, pathSet);
      if (cycle != null) {
        return cycle;
      }
    }

    path.removeLast();
    pathSet.remove(node);
    visited[node] = _VisitState.visited;
    return null;
  }
}

enum _VisitState {
  visiting,
  visited,
}

int _compareEdges(DirectedEdge left, DirectedEdge right) {
  final byFrom = left.from.compareTo(right.from);
  if (byFrom != 0) {
    return byFrom;
  }
  return left.to.compareTo(right.to);
}
