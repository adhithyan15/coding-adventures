import 'dart:collection';
import 'dart:convert';

import 'errors.dart';

enum GraphRepr {
  adjacencyList,
  adjacencyMatrix,
}

typedef WeightedEdge<T> = ({T left, T right, double weight});

int compareNodes<T>(T left, T right) {
  if (left is String && right is String) {
    return left.compareTo(right);
  }
  if (left is num && right is num) {
    return left.compareTo(right);
  }
  if (left is bool && right is bool) {
    if (left == right) {
      return 0;
    }
    return left ? 1 : -1;
  }

  return _nodeSortKey(left).compareTo(_nodeSortKey(right));
}

class Graph<T> {
  Graph([GraphRepr repr = GraphRepr.adjacencyList]) : _repr = repr;

  final GraphRepr _repr;
  final Map<T, Map<T, double>> _adj = <T, Map<T, double>>{};
  final List<T> _nodeList = <T>[];
  final Map<T, int> _nodeIndex = <T, int>{};
  final List<List<double?>> _matrix = <List<double?>>[];

  GraphRepr get repr => _repr;

  int get size =>
      _repr == GraphRepr.adjacencyList ? _adj.length : _nodeList.length;

  void addNode(T node) {
    if (_repr == GraphRepr.adjacencyList) {
      _adj.putIfAbsent(node, () => <T, double>{});
      return;
    }

    if (_nodeIndex.containsKey(node)) {
      return;
    }

    final index = _nodeList.length;
    _nodeList.add(node);
    _nodeIndex[node] = index;

    for (final row in _matrix) {
      row.add(null);
    }
    _matrix.add(List<double?>.filled(index + 1, null, growable: true));
  }

  void removeNode(T node) {
    if (_repr == GraphRepr.adjacencyList) {
      final neighbors = _adj[node];
      if (neighbors == null) {
        throw NodeNotFoundError<T>(node);
      }

      for (final neighbor in List<T>.from(neighbors.keys)) {
        _adj[neighbor]?.remove(node);
      }
      _adj.remove(node);
      return;
    }

    final index = _nodeIndex.remove(node);
    if (index == null) {
      throw NodeNotFoundError<T>(node);
    }

    _nodeList.removeAt(index);
    _matrix.removeAt(index);
    for (final row in _matrix) {
      row.removeAt(index);
    }

    for (var i = index; i < _nodeList.length; i++) {
      _nodeIndex[_nodeList[i]] = i;
    }
  }

  bool hasNode(T node) => _repr == GraphRepr.adjacencyList
      ? _adj.containsKey(node)
      : _nodeIndex.containsKey(node);

  Set<T> nodes() {
    final nodes = _repr == GraphRepr.adjacencyList
        ? _adj.keys.toList()
        : List<T>.from(_nodeList);
    nodes.sort(compareNodes);
    return Set<T>.unmodifiable(nodes);
  }

  void addEdge(T left, T right, [num weight = 1]) {
    final normalizedWeight = weight.toDouble();
    addNode(left);
    addNode(right);

    if (_repr == GraphRepr.adjacencyList) {
      _adj[left]![right] = normalizedWeight;
      _adj[right]![left] = normalizedWeight;
      return;
    }

    final leftIndex = _nodeIndex[left]!;
    final rightIndex = _nodeIndex[right]!;
    _matrix[leftIndex][rightIndex] = normalizedWeight;
    _matrix[rightIndex][leftIndex] = normalizedWeight;
  }

  void removeEdge(T left, T right) {
    if (_repr == GraphRepr.adjacencyList) {
      final leftNeighbors = _adj[left];
      final rightNeighbors = _adj[right];
      if (leftNeighbors == null ||
          rightNeighbors == null ||
          !leftNeighbors.containsKey(right)) {
        throw EdgeNotFoundError<T>(left, right);
      }

      leftNeighbors.remove(right);
      if (left != right) {
        rightNeighbors.remove(left);
      }
      return;
    }

    final leftIndex = _nodeIndex[left];
    final rightIndex = _nodeIndex[right];
    if (leftIndex == null ||
        rightIndex == null ||
        _matrix[leftIndex][rightIndex] == null) {
      throw EdgeNotFoundError<T>(left, right);
    }

    _matrix[leftIndex][rightIndex] = null;
    _matrix[rightIndex][leftIndex] = null;
  }

  bool hasEdge(T left, T right) {
    if (_repr == GraphRepr.adjacencyList) {
      return _adj[left]?.containsKey(right) ?? false;
    }

    final leftIndex = _nodeIndex[left];
    final rightIndex = _nodeIndex[right];
    if (leftIndex == null || rightIndex == null) {
      return false;
    }

    return _matrix[leftIndex][rightIndex] != null;
  }

  double edgeWeight(T left, T right) {
    if (_repr == GraphRepr.adjacencyList) {
      final weight = _adj[left]?[right];
      if (weight == null) {
        throw EdgeNotFoundError<T>(left, right);
      }
      return weight;
    }

    final leftIndex = _nodeIndex[left];
    final rightIndex = _nodeIndex[right];
    if (leftIndex == null || rightIndex == null) {
      throw EdgeNotFoundError<T>(left, right);
    }

    final weight = _matrix[leftIndex][rightIndex];
    if (weight == null) {
      throw EdgeNotFoundError<T>(left, right);
    }
    return weight;
  }

  List<WeightedEdge<T>> edges() {
    final result = <WeightedEdge<T>>[];

    if (_repr == GraphRepr.adjacencyList) {
      final seen = <String>{};
      for (final entry in _adj.entries) {
        final left = entry.key;
        for (final neighborEntry in entry.value.entries) {
          final right = neighborEntry.key;
          final key = _edgeKey(left, right);
          if (!seen.add(key)) {
            continue;
          }
          result.add(_canonicalEdge(left, right, neighborEntry.value));
        }
      }
    } else {
      for (var row = 0; row < _nodeList.length; row++) {
        for (var col = row; col < _nodeList.length; col++) {
          final weight = _matrix[row][col];
          if (weight != null) {
            result.add(
              _canonicalEdge(_nodeList[row], _nodeList[col], weight),
            );
          }
        }
      }
    }

    result.sort(_compareEdges);
    return List<WeightedEdge<T>>.unmodifiable(result);
  }

  Set<T> neighbors(T node) {
    if (_repr == GraphRepr.adjacencyList) {
      final neighbors = _adj[node];
      if (neighbors == null) {
        throw NodeNotFoundError<T>(node);
      }

      final sorted = neighbors.keys.toList()..sort(compareNodes);
      return Set<T>.unmodifiable(sorted);
    }

    final index = _nodeIndex[node];
    if (index == null) {
      throw NodeNotFoundError<T>(node);
    }

    final neighbors = <T>[];
    for (var col = 0; col < _nodeList.length; col++) {
      if (_matrix[index][col] != null) {
        neighbors.add(_nodeList[col]);
      }
    }
    neighbors.sort(compareNodes);
    return Set<T>.unmodifiable(neighbors);
  }

  Map<T, double> neighborsWeighted(T node) {
    if (_repr == GraphRepr.adjacencyList) {
      final neighbors = _adj[node];
      if (neighbors == null) {
        throw NodeNotFoundError<T>(node);
      }

      final entries = neighbors.entries.toList()
        ..sort((left, right) => compareNodes(left.key, right.key));
      return Map<T, double>.unmodifiable(
        LinkedHashMap<T, double>.fromEntries(entries),
      );
    }

    final index = _nodeIndex[node];
    if (index == null) {
      throw NodeNotFoundError<T>(node);
    }

    final entries = <MapEntry<T, double>>[];
    for (var col = 0; col < _nodeList.length; col++) {
      final weight = _matrix[index][col];
      if (weight != null) {
        entries.add(MapEntry<T, double>(_nodeList[col], weight));
      }
    }
    entries.sort((left, right) => compareNodes(left.key, right.key));
    return Map<T, double>.unmodifiable(
      LinkedHashMap<T, double>.fromEntries(entries),
    );
  }

  int degree(T node) => neighbors(node).length;

  @override
  String toString() => 'Graph(nodes=$size, edges=${edges().length}, repr=$_repr)';
}

String _nodeSortKey(Object? node) {
  if (node is String) {
    return 'string:$node';
  }
  if (node is num) {
    return 'number:$node';
  }
  if (node is bool) {
    return 'bool:$node';
  }
  if (node == null) {
    return 'null:null';
  }

  try {
    return 'json:${jsonEncode(node)}';
  } catch (_) {
    return 'string:$node';
  }
}

String _edgeKey<T>(T left, T right) {
  final leftKey = _nodeSortKey(left);
  final rightKey = _nodeSortKey(right);
  return leftKey.compareTo(rightKey) <= 0
      ? '$leftKey\0$rightKey'
      : '$rightKey\0$leftKey';
}

WeightedEdge<T> _canonicalEdge<T>(T left, T right, double weight) {
  if (compareNodes(left, right) <= 0) {
    return (left: left, right: right, weight: weight);
  }
  return (left: right, right: left, weight: weight);
}

int _compareEdges<T>(WeightedEdge<T> left, WeightedEdge<T> right) {
  final byWeight = left.weight.compareTo(right.weight);
  if (byWeight != 0) {
    return byWeight;
  }

  final byLeft = compareNodes(left.left, right.left);
  if (byLeft != 0) {
    return byLeft;
  }

  return compareNodes(left.right, right.right);
}
