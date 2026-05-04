import 'dart:collection';
import 'dart:convert';

import 'errors.dart';

enum GraphRepr { adjacencyList, adjacencyMatrix }

typedef WeightedEdge<T> = ({T left, T right, double weight});
typedef GraphPropertyValue = Object?;
typedef GraphPropertyBag = Map<String, GraphPropertyValue>;

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
  final GraphPropertyBag _graphProperties = <String, GraphPropertyValue>{};
  final Map<T, GraphPropertyBag> _nodeProperties = <T, GraphPropertyBag>{};
  final Map<String, GraphPropertyBag> _edgeProperties =
      <String, GraphPropertyBag>{};

  GraphRepr get repr => _repr;

  int get size =>
      _repr == GraphRepr.adjacencyList ? _adj.length : _nodeList.length;

  void addNode(T node, [GraphPropertyBag properties = const {}]) {
    if (_repr == GraphRepr.adjacencyList) {
      _adj.putIfAbsent(node, () => <T, double>{});
      _mergeNodeProperties(node, properties);
      return;
    }

    if (_nodeIndex.containsKey(node)) {
      _mergeNodeProperties(node, properties);
      return;
    }

    final index = _nodeList.length;
    _nodeList.add(node);
    _nodeIndex[node] = index;

    for (final row in _matrix) {
      row.add(null);
    }
    _matrix.add(List<double?>.filled(index + 1, null, growable: true));
    _mergeNodeProperties(node, properties);
  }

  void removeNode(T node) {
    if (_repr == GraphRepr.adjacencyList) {
      final neighbors = _adj[node];
      if (neighbors == null) {
        throw NodeNotFoundError<T>(node);
      }

      for (final neighbor in List<T>.from(neighbors.keys)) {
        _adj[neighbor]?.remove(node);
        _edgeProperties.remove(_edgeKey(node, neighbor));
      }
      _adj.remove(node);
      _nodeProperties.remove(node);
      return;
    }

    final index = _nodeIndex.remove(node);
    if (index == null) {
      throw NodeNotFoundError<T>(node);
    }

    for (final other in _nodeList) {
      _edgeProperties.remove(_edgeKey(node, other));
    }
    _nodeProperties.remove(node);
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

  void addEdge(
    T left,
    T right, [
    num weight = 1,
    GraphPropertyBag properties = const {},
  ]) {
    final normalizedWeight = weight.toDouble();
    addNode(left);
    addNode(right);

    if (_repr == GraphRepr.adjacencyList) {
      _adj[left]![right] = normalizedWeight;
      _adj[right]![left] = normalizedWeight;
      _mergeEdgeProperties(left, right, normalizedWeight, properties);
      return;
    }

    final leftIndex = _nodeIndex[left]!;
    final rightIndex = _nodeIndex[right]!;
    _matrix[leftIndex][rightIndex] = normalizedWeight;
    _matrix[rightIndex][leftIndex] = normalizedWeight;
    _mergeEdgeProperties(left, right, normalizedWeight, properties);
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
      _edgeProperties.remove(_edgeKey(left, right));
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
    _edgeProperties.remove(_edgeKey(left, right));
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
            result.add(_canonicalEdge(_nodeList[row], _nodeList[col], weight));
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

  GraphPropertyBag graphProperties() =>
      Map<String, GraphPropertyValue>.unmodifiable(_graphProperties);

  void setGraphProperty(String key, GraphPropertyValue value) {
    _graphProperties[key] = value;
  }

  void removeGraphProperty(String key) {
    _graphProperties.remove(key);
  }

  GraphPropertyBag nodeProperties(T node) {
    if (!hasNode(node)) {
      throw NodeNotFoundError<T>(node);
    }

    return Map<String, GraphPropertyValue>.unmodifiable(
      _nodeProperties[node] ?? const <String, GraphPropertyValue>{},
    );
  }

  void setNodeProperty(T node, String key, GraphPropertyValue value) {
    if (!hasNode(node)) {
      throw NodeNotFoundError<T>(node);
    }

    _nodeProperties.putIfAbsent(node, () => <String, GraphPropertyValue>{});
    _nodeProperties[node]![key] = value;
  }

  void removeNodeProperty(T node, String key) {
    if (!hasNode(node)) {
      throw NodeNotFoundError<T>(node);
    }

    _nodeProperties[node]?.remove(key);
  }

  GraphPropertyBag edgeProperties(T left, T right) {
    if (!hasEdge(left, right)) {
      throw EdgeNotFoundError<T>(left, right);
    }

    final properties = <String, GraphPropertyValue>{
      ...?_edgeProperties[_edgeKey(left, right)],
      'weight': edgeWeight(left, right),
    };
    return Map<String, GraphPropertyValue>.unmodifiable(properties);
  }

  void setEdgeProperty(T left, T right, String key, GraphPropertyValue value) {
    if (!hasEdge(left, right)) {
      throw EdgeNotFoundError<T>(left, right);
    }

    if (key == 'weight') {
      if (value is! num) {
        throw ArgumentError("edge property 'weight' must be numeric");
      }
      _setEdgeWeight(left, right, value.toDouble());
    }

    _edgeProperties.putIfAbsent(
      _edgeKey(left, right),
      () => <String, GraphPropertyValue>{},
    );
    _edgeProperties[_edgeKey(left, right)]![key] = value;
  }

  void removeEdgeProperty(T left, T right, String key) {
    if (!hasEdge(left, right)) {
      throw EdgeNotFoundError<T>(left, right);
    }

    if (key == 'weight') {
      _setEdgeWeight(left, right, 1);
      _edgeProperties.putIfAbsent(
        _edgeKey(left, right),
        () => <String, GraphPropertyValue>{},
      );
      _edgeProperties[_edgeKey(left, right)]!['weight'] = 1.0;
      return;
    }

    _edgeProperties[_edgeKey(left, right)]?.remove(key);
  }

  @override
  String toString() =>
      'Graph(nodes=$size, edges=${edges().length}, repr=$_repr)';

  void _mergeNodeProperties(T node, GraphPropertyBag properties) {
    _nodeProperties.putIfAbsent(node, () => <String, GraphPropertyValue>{});
    _nodeProperties[node]!.addAll(properties);
  }

  void _mergeEdgeProperties(
    T left,
    T right,
    double weight,
    GraphPropertyBag properties,
  ) {
    final key = _edgeKey(left, right);
    _edgeProperties.putIfAbsent(key, () => <String, GraphPropertyValue>{});
    _edgeProperties[key]!.addAll(properties);
    _edgeProperties[key]!['weight'] = weight;
  }

  void _setEdgeWeight(T left, T right, double weight) {
    if (_repr == GraphRepr.adjacencyList) {
      _adj[left]![right] = weight;
      _adj[right]![left] = weight;
      return;
    }

    final leftIndex = _nodeIndex[left]!;
    final rightIndex = _nodeIndex[right]!;
    _matrix[leftIndex][rightIndex] = weight;
    _matrix[rightIndex][leftIndex] = weight;
  }
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
