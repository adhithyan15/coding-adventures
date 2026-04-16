import 'dart:collection';

import 'errors.dart';
import 'graph.dart';

List<T> bfs<T>(Graph<T> graph, T start) {
  if (!graph.hasNode(start)) {
    throw NodeNotFoundError<T>(start);
  }

  final visited = <T>{start};
  final queue = ListQueue<T>()..add(start);
  final result = <T>[];

  while (queue.isNotEmpty) {
    final node = queue.removeFirst();
    result.add(node);

    for (final neighbor in _sortedNodes(graph.neighbors(node))) {
      if (visited.add(neighbor)) {
        queue.addLast(neighbor);
      }
    }
  }

  return List<T>.unmodifiable(result);
}

List<T> dfs<T>(Graph<T> graph, T start) {
  if (!graph.hasNode(start)) {
    throw NodeNotFoundError<T>(start);
  }

  final visited = <T>{};
  final stack = <T>[start];
  final result = <T>[];

  while (stack.isNotEmpty) {
    final node = stack.removeLast();
    if (!visited.add(node)) {
      continue;
    }

    result.add(node);

    for (final neighbor in _sortedNodes(graph.neighbors(node)).reversed) {
      if (!visited.contains(neighbor)) {
        stack.add(neighbor);
      }
    }
  }

  return List<T>.unmodifiable(result);
}

bool isConnected<T>(Graph<T> graph) {
  if (graph.size == 0) {
    return true;
  }

  final start = _sortedNodes(graph.nodes()).first;
  return bfs(graph, start).length == graph.size;
}

List<Set<T>> connectedComponents<T>(Graph<T> graph) {
  final unvisited = graph.nodes().toSet();
  final result = <Set<T>>[];

  while (unvisited.isNotEmpty) {
    final start = _sortedNodes(unvisited).first;
    final component = bfs(graph, start).toSet();
    result.add(Set<T>.unmodifiable(component));
    for (final node in component) {
      unvisited.remove(node);
    }
  }

  return List<Set<T>>.unmodifiable(result);
}

bool hasCycle<T>(Graph<T> graph) {
  final visited = <T>{};

  for (final start in _sortedNodes(graph.nodes())) {
    if (visited.contains(start)) {
      continue;
    }

    final stack = <_CycleFrame<T>>[_CycleFrame<T>(node: start)];
    while (stack.isNotEmpty) {
      final frame = stack.removeLast();
      if (visited.contains(frame.node)) {
        continue;
      }

      visited.add(frame.node);
      for (final neighbor in _sortedNodes(graph.neighbors(frame.node))) {
        if (!visited.contains(neighbor)) {
          stack.add(
            _CycleFrame<T>(
              node: neighbor,
              parent: frame.node,
              hasParent: true,
            ),
          );
        } else if (!frame.hasParent || neighbor != frame.parent) {
          return true;
        }
      }
    }
  }

  return false;
}

List<T> shortestPath<T>(Graph<T> graph, T start, T end) {
  if (!graph.hasNode(start) || !graph.hasNode(end)) {
    return <T>[];
  }
  if (start == end) {
    return List<T>.unmodifiable(<T>[start]);
  }

  final allUnit = graph.edges().every((edge) => edge.weight == 1.0);
  return allUnit
      ? _bfsShortestPath(graph, start, end)
      : _dijkstraShortestPath(graph, start, end);
}

Set<WeightedEdge<T>> minimumSpanningTree<T>(Graph<T> graph) {
  final nodes = _sortedNodes(graph.nodes());
  final edges = graph.edges();
  if (nodes.length <= 1 || edges.isEmpty) {
    return <WeightedEdge<T>>{};
  }
  if (!isConnected(graph)) {
    throw const GraphNotConnectedError();
  }

  final unionFind = _UnionFind<T>(nodes);
  final mst = LinkedHashSet<WeightedEdge<T>>();
  for (final edge in edges) {
    if (unionFind.find(edge.left) != unionFind.find(edge.right)) {
      unionFind.union(edge.left, edge.right);
      mst.add(edge);
      if (mst.length == nodes.length - 1) {
        break;
      }
    }
  }

  return Set<WeightedEdge<T>>.unmodifiable(mst);
}

List<T> _bfsShortestPath<T>(Graph<T> graph, T start, T end) {
  final parent = <T, T>{};
  final visited = <T>{start};
  final queue = ListQueue<T>()..add(start);

  while (queue.isNotEmpty) {
    final node = queue.removeFirst();
    if (node == end) {
      break;
    }

    for (final neighbor in _sortedNodes(graph.neighbors(node))) {
      if (visited.add(neighbor)) {
        parent[neighbor] = node;
        queue.addLast(neighbor);
      }
    }
  }

  if (!visited.contains(end)) {
    return <T>[];
  }

  return _reconstructPath(parent, start, end);
}

List<T> _dijkstraShortestPath<T>(Graph<T> graph, T start, T end) {
  final distances = <T, double>{};
  final parent = <T, T>{};
  final queue = _MinPriorityQueue<T>();
  var sequence = 0;

  for (final node in graph.nodes()) {
    distances[node] = double.infinity;
  }

  distances[start] = 0;
  queue.push(priority: 0, sequence: sequence, value: start);

  while (queue.isNotEmpty) {
    final current = queue.pop()!;
    final currentDistance = distances[current.value] ?? double.infinity;
    if (current.priority > currentDistance) {
      continue;
    }
    if (current.value == end) {
      break;
    }

    final neighbors = graph.neighborsWeighted(current.value).entries.toList()
      ..sort((left, right) => compareNodes(left.key, right.key));

    for (final entry in neighbors) {
      final nextDistance = currentDistance + entry.value;
      if (nextDistance < (distances[entry.key] ?? double.infinity)) {
        distances[entry.key] = nextDistance;
        parent[entry.key] = current.value;
        sequence += 1;
        queue.push(
          priority: nextDistance,
          sequence: sequence,
          value: entry.key,
        );
      }
    }
  }

  if ((distances[end] ?? double.infinity) == double.infinity) {
    return <T>[];
  }

  return _reconstructPath(parent, start, end);
}

List<T> _reconstructPath<T>(Map<T, T> parent, T start, T end) {
  final path = <T>[];
  var current = end;
  while (true) {
    path.add(current);
    if (current == start) {
      break;
    }

    final previous = parent[current];
    if (previous == null) {
      return <T>[];
    }
    current = previous;
  }

  return List<T>.unmodifiable(path.reversed.toList());
}

List<T> _sortedNodes<T>(Iterable<T> nodes) {
  final sorted = nodes.toList();
  sorted.sort(compareNodes);
  return sorted;
}

class _CycleFrame<T> {
  _CycleFrame({
    required this.node,
    this.parent,
    this.hasParent = false,
  });

  final T node;
  final T? parent;
  final bool hasParent;
}

class _PriorityQueueEntry<T> {
  const _PriorityQueueEntry({
    required this.priority,
    required this.sequence,
    required this.value,
  });

  final double priority;
  final int sequence;
  final T value;
}

class _MinPriorityQueue<T> {
  final List<_PriorityQueueEntry<T>> _items = <_PriorityQueueEntry<T>>[];

  bool get isNotEmpty => _items.isNotEmpty;

  void push({
    required double priority,
    required int sequence,
    required T value,
  }) {
    _items.add(
      _PriorityQueueEntry<T>(
        priority: priority,
        sequence: sequence,
        value: value,
      ),
    );
    _bubbleUp(_items.length - 1);
  }

  _PriorityQueueEntry<T>? pop() {
    if (_items.isEmpty) {
      return null;
    }

    final top = _items.first;
    final last = _items.removeLast();
    if (_items.isNotEmpty) {
      _items[0] = last;
      _bubbleDown(0);
    }
    return top;
  }

  void _bubbleUp(int index) {
    var currentIndex = index;
    while (currentIndex > 0) {
      final parentIndex = (currentIndex - 1) ~/ 2;
      if (_compare(_items[parentIndex], _items[currentIndex]) <= 0) {
        break;
      }

      final tmp = _items[parentIndex];
      _items[parentIndex] = _items[currentIndex];
      _items[currentIndex] = tmp;
      currentIndex = parentIndex;
    }
  }

  void _bubbleDown(int index) {
    var currentIndex = index;
    while (true) {
      var smallest = currentIndex;
      final left = currentIndex * 2 + 1;
      final right = currentIndex * 2 + 2;

      if (left < _items.length &&
          _compare(_items[left], _items[smallest]) < 0) {
        smallest = left;
      }
      if (right < _items.length &&
          _compare(_items[right], _items[smallest]) < 0) {
        smallest = right;
      }
      if (smallest == currentIndex) {
        return;
      }

      final tmp = _items[currentIndex];
      _items[currentIndex] = _items[smallest];
      _items[smallest] = tmp;
      currentIndex = smallest;
    }
  }

  int _compare(
    _PriorityQueueEntry<T> left,
    _PriorityQueueEntry<T> right,
  ) {
    final byPriority = left.priority.compareTo(right.priority);
    if (byPriority != 0) {
      return byPriority;
    }
    return left.sequence.compareTo(right.sequence);
  }
}

class _UnionFind<T> {
  _UnionFind(Iterable<T> nodes) {
    for (final node in nodes) {
      _parent[node] = node;
      _rank[node] = 0;
    }
  }

  final Map<T, T> _parent = <T, T>{};
  final Map<T, int> _rank = <T, int>{};

  T find(T node) {
    final parent = _parent[node];
    if (parent == null) {
      throw NodeNotFoundError<T>(node);
    }
    if (parent != node) {
      _parent[node] = find(parent);
    }
    return _parent[node] as T;
  }

  void union(T left, T right) {
    var leftRoot = find(left);
    var rightRoot = find(right);
    if (leftRoot == rightRoot) {
      return;
    }

    final leftRank = _rank[leftRoot] ?? 0;
    final rightRank = _rank[rightRoot] ?? 0;
    if (leftRank < rightRank) {
      final tmp = leftRoot;
      leftRoot = rightRoot;
      rightRoot = tmp;
    }

    _parent[rightRoot] = leftRoot;
    if (leftRank == rightRank) {
      _rank[leftRoot] = leftRank + 1;
    }
  }
}
