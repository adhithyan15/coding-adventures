import 'dart:collection';
import 'dart:math' as math;

import 'constants.dart';
import 'errors.dart';
import 'messages.dart';
import 'radix_entry.dart';

List<int> _checkedScalars(String input) {
  final scalars = <int>[];
  for (final scalar in input.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidRadixKeyException.invalidScalar();
    }
    scalars.add(scalar);
    if (scalars.length > maxKeyScalars) {
      throw const InvalidRadixKeyException.keyTooLong();
    }
  }
  return scalars;
}

final class _RadixEdge<V> {
  _RadixEdge(this.label, this.child);

  List<int> label;
  _RadixNode<V> child;
}

final class _RadixNode<V> {
  _RadixNode({this.isEnd = false, this.value});

  bool isEnd;
  V? value;
  final Map<int, _RadixEdge<V>> children = SplayTreeMap<int, _RadixEdge<V>>();
}

final class _CollectFrame<V> {
  _CollectFrame(this.node, this.path)
      : childKeys = node.children.keys.toList(growable: false);

  final _RadixNode<V> node;
  final List<int> path;
  final List<int> childKeys;
  var childIndex = 0;
  var checkedEnd = false;
}

final class _PathEntry<V> {
  _PathEntry(this.node, this.childEdgeFirst);

  final _RadixNode<V> node;
  final int childEdgeFirst;
}

/// A pure Dart DT14 radix tree (compressed Patricia trie) mapping string keys to values.
///
/// Features Unicode-scalar compressed edges, iterative stack-safe traversal,
/// merge-back re-compression on deletion, and payload-blind diagnostic safety.
final class RadixTree<V> {
  /// Create an empty radix tree.
  RadixTree();

  /// Create a radix tree initialized with [entries].
  RadixTree.fromEntries(Iterable<RadixEntry<V>> entries) {
    for (final entry in entries) {
      insert(entry.key, entry.value);
    }
  }

  /// Create a radix tree initialized with key-value pairs from [map].
  RadixTree.fromMap(Map<String, V> map) {
    for (final MapEntry(:key, :value) in map.entries) {
      insert(key, value);
    }
  }

  final _RadixNode<V> _root = _RadixNode<V>();
  var _count = 0;

  /// The number of distinct stored keys in the tree.
  int get count => _count;

  /// Alias for [count].
  int get size => _count;

  /// Whether the tree contains zero keys.
  bool get isEmpty => _count == 0;

  /// Whether the tree contains at least one key.
  bool get isNotEmpty => _count > 0;

  /// Total number of nodes currently allocated in the tree.
  int nodeCount() {
    var count = 0;
    final stack = <_RadixNode<V>>[_root];
    while (stack.isNotEmpty) {
      final node = stack.removeLast();
      count++;
      for (final edge in node.children.values) {
        stack.add(edge.child);
      }
    }
    return count;
  }

  /// Total number of compressed edges currently in the tree.
  int edgeCount() {
    var count = 0;
    final stack = <_RadixNode<V>>[_root];
    while (stack.isNotEmpty) {
      final node = stack.removeLast();
      count += node.children.length;
      for (final edge in node.children.values) {
        stack.add(edge.child);
      }
    }
    return count;
  }

  /// Insert [key] with [value], returning `true` if a new key was inserted or `false` if updated.
  ///
  /// Handles all 4 DT14 insertion cases iteratively without recursive call-stack consumption.
  bool insert(String key, V value) {
    final scalars = _checkedScalars(key);
    if (scalars.isEmpty) {
      final wasNew = !_root.isEnd;
      _root.isEnd = true;
      _root.value = value;
      if (wasNew) _count++;
      return wasNew;
    }

    var currNode = _root;
    var offset = 0;

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];

      if (edge == null) {
        // Case 1: No common prefix — create new edge directly.
        final remaining = scalars.sublist(offset);
        final childNode = _RadixNode<V>(isEnd: true, value: value);
        currNode.children[first] = _RadixEdge<V>(remaining, childNode);
        _count++;
        return true;
      }

      var common = 0;
      final maxCommon = math.min(scalars.length - offset, edge.label.length);
      while (common < maxCommon &&
          scalars[offset + common] == edge.label[common]) {
        common++;
      }

      if (common == edge.label.length) {
        if (offset + common == scalars.length) {
          // Key ends exactly at existing child node.
          final wasNew = !edge.child.isEnd;
          edge.child.isEnd = true;
          edge.child.value = value;
          if (wasNew) _count++;
          return wasNew;
        }
        // Case 2: Edge label is a prefix of remaining key — descend into child.
        currNode = edge.child;
        offset += common;
      } else if (common == scalars.length - offset) {
        // Case 3: Remaining key is a strict prefix of edge label — split edge.
        final remainingOldLabel = edge.label.sublist(common);
        final splitNode = _RadixNode<V>(isEnd: true, value: value);
        splitNode.children[remainingOldLabel[0]] =
            _RadixEdge<V>(remainingOldLabel, edge.child);
        currNode.children[first] =
            _RadixEdge<V>(edge.label.sublist(0, common), splitNode);
        _count++;
        return true;
      } else {
        // Case 4: Partial match — split at divergence point.
        final remainingOldLabel = edge.label.sublist(common);
        final remainingNewKey = scalars.sublist(offset + common);
        final splitNode = _RadixNode<V>(isEnd: false, value: null);
        splitNode.children[remainingOldLabel[0]] =
            _RadixEdge<V>(remainingOldLabel, edge.child);
        final newLeaf = _RadixNode<V>(isEnd: true, value: value);
        splitNode.children[remainingNewKey[0]] =
            _RadixEdge<V>(remainingNewKey, newLeaf);
        currNode.children[first] =
            _RadixEdge<V>(edge.label.sublist(0, common), splitNode);
        _count++;
        return true;
      }
    }

    return false;
  }

  /// Store [key] with [value]. Alias for [insert].
  bool put(String key, V value) => insert(key, value);

  /// Find the value associated with [key], or `null` if not found.
  ///
  /// For nullable value types (`RadixTree<V?>`), use [containsKey] to distinguish
  /// between a stored `null` and an absent key.
  V? search(String key) {
    final scalars = _checkedScalars(key);
    final node = _findNode(scalars);
    return (node == null || !node.isEnd) ? null : node.value;
  }

  /// Alias for [search].
  V? get(String key) => search(key);

  /// Whether [key] is present in the tree.
  bool containsKey(String key) {
    final scalars = _checkedScalars(key);
    final node = _findNode(scalars);
    return node != null && node.isEnd;
  }

  /// Retrieve the value associated with [key], throwing [RadixKeyNotFoundException] if absent.
  V operator [](String key) {
    final scalars = _checkedScalars(key);
    final node = _findNode(scalars);
    if (node == null || !node.isEnd) {
      throw const RadixKeyNotFoundException();
    }
    return node.value as V;
  }

  /// Insert or update [key] with [value].
  void operator []=(String key, V value) => insert(key, value);

  /// Delete [key] from the tree, performing merge-back re-compression.
  ///
  /// Returns `true` if the key was deleted, or `false` if it was not present.
  bool delete(String key) {
    final scalars = _checkedScalars(key);
    if (scalars.isEmpty) {
      if (!_root.isEnd) return false;
      _root.isEnd = false;
      _root.value = null;
      _count--;
      return true;
    }

    final pathStack = <_PathEntry<V>>[];
    var currNode = _root;
    var offset = 0;

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];
      if (edge == null) return false;

      if (offset + edge.label.length > scalars.length) return false;
      for (var i = 0; i < edge.label.length; i++) {
        if (scalars[offset + i] != edge.label[i]) return false;
      }

      pathStack.add(_PathEntry(currNode, first));
      currNode = edge.child;
      offset += edge.label.length;
    }

    if (!currNode.isEnd) return false;

    currNode.isEnd = false;
    currNode.value = null;
    _count--;

    var childNode = currNode;
    while (pathStack.isNotEmpty) {
      final entry = pathStack.removeLast();
      final parentNode = entry.node;
      final first = entry.childEdgeFirst;
      final edge = parentNode.children[first]!;

      if (childNode.children.isEmpty && !childNode.isEnd) {
        parentNode.children.remove(first);
      } else if (!childNode.isEnd &&
          childNode.children.length == singleChildThreshold) {
        final grandChildEdge = childNode.children.values.first;
        final mergedLabel = <int>[...edge.label, ...grandChildEdge.label];
        parentNode.children[first] =
            _RadixEdge<V>(mergedLabel, grandChildEdge.child);
      }

      childNode = parentNode;
    }

    return true;
  }

  /// Whether any stored key begins with [prefix].
  bool startsWith(String prefix) {
    final scalars = _checkedScalars(prefix);
    if (scalars.isEmpty) return _count > 0;

    var currNode = _root;
    var offset = 0;

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];
      if (edge == null) return false;

      var common = 0;
      final maxCommon = math.min(scalars.length - offset, edge.label.length);
      while (common < maxCommon &&
          scalars[offset + common] == edge.label[common]) {
        common++;
      }

      if (common == scalars.length - offset) {
        return true;
      }
      if (common < edge.label.length) {
        return false;
      }

      currNode = edge.child;
      offset += edge.label.length;
    }

    return currNode.isEnd || currNode.children.isNotEmpty;
  }

  /// All entries starting with [prefix], ordered by Unicode scalar values.
  List<RadixEntry<V>> wordsWithPrefix(String prefix) {
    final scalars = _checkedScalars(prefix);
    if (scalars.isEmpty) {
      return _collectSubtree(_root, const <int>[]);
    }

    var currNode = _root;
    var offset = 0;
    final matchedPath = <int>[];

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];
      if (edge == null) {
        return List<RadixEntry<V>>.unmodifiable(const <RadixEntry<Never>>[]);
      }

      var common = 0;
      final maxCommon = math.min(scalars.length - offset, edge.label.length);
      while (common < maxCommon &&
          scalars[offset + common] == edge.label[common]) {
        common++;
      }

      if (offset + common == scalars.length) {
        final fullPath = <int>[...matchedPath, ...edge.label];
        return _collectSubtree(edge.child, fullPath);
      }

      if (common < edge.label.length) {
        return List<RadixEntry<V>>.unmodifiable(const <RadixEntry<Never>>[]);
      }

      matchedPath.addAll(edge.label);
      currNode = edge.child;
      offset += edge.label.length;
    }

    return _collectSubtree(currNode, matchedPath);
  }

  /// All keys starting with [prefix], ordered by Unicode scalar values.
  List<String> keysWithPrefix(String prefix) =>
      List<String>.unmodifiable(wordsWithPrefix(prefix).map((e) => e.key));

  /// All entries stored in the tree, ordered by Unicode scalar values.
  List<RadixEntry<V>> allWords() => wordsWithPrefix('');

  /// Sorted snapshot of all stored keys.
  List<String> get keys => keysWithPrefix('');

  /// Sorted snapshot of all stored entries.
  List<RadixEntry<V>> get entries => allWords();

  /// Sorted map containing all stored entries.
  Map<String, V> toMap() => Map<String, V>.unmodifiable({
        for (final entry in allWords()) entry.key: entry.value,
      });

  /// Deepest stored key and value that forms a prefix of [text], or `null` if none.
  RadixEntry<V>? longestPrefixMatch(String text) {
    final scalars = _checkedScalars(text);
    var currNode = _root;
    var offset = 0;
    int? bestLength;
    V? bestValue;

    if (_root.isEnd) {
      bestLength = 0;
      bestValue = _root.value;
    }

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];
      if (edge == null) break;

      var common = 0;
      final maxCommon = math.min(scalars.length - offset, edge.label.length);
      while (common < maxCommon &&
          scalars[offset + common] == edge.label[common]) {
        common++;
      }

      if (common < edge.label.length) {
        break;
      }

      offset += edge.label.length;
      currNode = edge.child;
      if (currNode.isEnd) {
        bestLength = offset;
        bestValue = currNode.value;
      }
    }

    if (bestLength == null) return null;
    return RadixEntry<V>(
      String.fromCharCodes(scalars.take(bestLength)),
      bestValue as V,
    );
  }

  /// Deepest stored key that forms a prefix of [text], or `null` if none.
  String? longestPrefixOf(String text) => longestPrefixMatch(text)?.key;

  /// Verifies structural invariants: endpoint count, compression, scalar integrity.
  ///
  /// Returns `true` if all radix tree invariants are satisfied.
  bool isValid() {
    final stack = <(_RadixNode<V>, bool)>[(_root, true)];
    var endpoints = 0;

    while (stack.isNotEmpty) {
      final (currNode, isRoot) = stack.removeLast();

      if (currNode.isEnd) {
        endpoints++;
      } else {
        if (currNode.value != null) return false;
        if (!isRoot && currNode.children.length < minNonRootInnerChildren) {
          return false;
        }
      }

      for (final MapEntry(:key, :value) in currNode.children.entries) {
        final edge = value;
        if (edge.label.isEmpty) return false;
        if (edge.label[0] != key) return false;

        for (final scalar in edge.label) {
          if (scalar < minUnicodeScalar ||
              scalar > maxUnicodeScalar ||
              (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
            return false;
          }
        }

        stack.add((edge.child, false));
      }
    }

    return endpoints == _count;
  }

  _RadixNode<V>? _findNode(List<int> scalars) {
    var currNode = _root;
    var offset = 0;

    while (offset < scalars.length) {
      final first = scalars[offset];
      final edge = currNode.children[first];
      if (edge == null) return null;

      if (offset + edge.label.length > scalars.length) {
        return null;
      }

      for (var i = 0; i < edge.label.length; i++) {
        if (scalars[offset + i] != edge.label[i]) {
          return null;
        }
      }

      currNode = edge.child;
      offset += edge.label.length;
    }

    return currNode;
  }

  List<RadixEntry<V>> _collectSubtree(_RadixNode<V> start, List<int> basePath) {
    final results = <RadixEntry<V>>[];
    final stack = <_CollectFrame<V>>[_CollectFrame(start, basePath)];

    while (stack.isNotEmpty) {
      final frame = stack.last;
      if (!frame.checkedEnd) {
        frame.checkedEnd = true;
        if (frame.node.isEnd) {
          results.add(RadixEntry<V>(
            String.fromCharCodes(frame.path),
            frame.node.value as V,
          ));
        }
      }

      if (frame.childIndex < frame.childKeys.length) {
        final edgeKey = frame.childKeys[frame.childIndex++];
        final edge = frame.node.children[edgeKey]!;
        final childPath = <int>[...frame.path, ...edge.label];
        stack.add(_CollectFrame(edge.child, childPath));
      } else {
        stack.removeLast();
      }
    }

    return List<RadixEntry<V>>.unmodifiable(results);
  }

  @override
  String toString() =>
      '$radixTreeStringPrefix$_count$radixTreeNodeCountPrefix${nodeCount()}$radixTreeEdgeCountPrefix${edgeCount()}$radixTreeStringSuffix';
}
