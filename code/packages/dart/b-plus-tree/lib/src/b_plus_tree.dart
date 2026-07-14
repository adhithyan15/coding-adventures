import 'dart:collection';

/// A sorted key-value entry returned by B+ tree scans.
typedef BPlusTreeEntry<K, V> = (K, V);

Comparator<T> _naturalOrder<T>() => (left, right) =>
    Comparable.compare(left as Comparable, right as Comparable);

sealed class _BPlusNode<K, V> {
  final List<K> keys = <K>[];
}

final class _BPlusInternalNode<K, V> extends _BPlusNode<K, V> {
  final List<_BPlusNode<K, V>> children = <_BPlusNode<K, V>>[];
}

final class _BPlusLeafNode<K, V> extends _BPlusNode<K, V> {
  final List<V> values = <V>[];
  _BPlusLeafNode<K, V>? next;

  int findKeyIndex(K key, Comparator<K> compare) {
    var low = 0;
    var high = keys.length;
    while (low < high) {
      final middle = (low + high) >> 1;
      if (compare(keys[middle], key) < 0) {
        low = middle + 1;
      } else {
        high = middle;
      }
    }
    return low;
  }
}

/// A generic B+ tree mapping ordered keys to values.
///
/// Data lives only in linked leaf nodes. Internal nodes contain the minimum key
/// of each right child as a routing separator. Inserting an existing key
/// updates its value, and deleting a missing key is a no-op that returns false.
final class BPlusTree<K, V> {
  /// Create an empty B+ tree with minimum degree [minimumDegree].
  ///
  /// Natural [Comparable] order is used unless [compare] is supplied.
  BPlusTree([this.minimumDegree = 2, Comparator<K>? compare])
      : _compare = compare ?? _naturalOrder<K>() {
    if (minimumDegree < 2) {
      throw ArgumentError.value(
        minimumDegree,
        'minimumDegree',
        'must be at least 2',
      );
    }
    _entries = SplayTreeMap<K, V>(_compare);
    final leaf = _BPlusLeafNode<K, V>();
    _root = leaf;
    _firstLeaf = leaf;
  }

  /// The minimum number of children in a non-root internal node.
  final int minimumDegree;
  final Comparator<K> _compare;
  late final SplayTreeMap<K, V> _entries;
  late _BPlusNode<K, V> _root;
  late _BPlusLeafNode<K, V> _firstLeaf;

  int get _maxKeys => 2 * minimumDegree - 1;
  int get _maxChildren => 2 * minimumDegree;

  /// Number of key-value pairs stored in the tree.
  int get count => _entries.length;

  /// Alias for [count], matching the DT12 vocabulary.
  int get size => count;

  /// Whether the tree contains no keys.
  bool get isEmpty => _entries.isEmpty;

  /// Number of edges from the root to any leaf.
  int get height {
    var node = _root;
    var result = 0;
    while (node is _BPlusInternalNode<K, V>) {
      result++;
      node = node.children.first;
    }
    return result;
  }

  /// Insert [key] with [value], updating an existing mapping in place.
  void insert(K key, V value) {
    _entries[key] = value;
    _rebuild();
  }

  /// Remove [key], returning whether it was present.
  bool delete(K key) {
    if (!_entries.containsKey(key)) return false;
    _entries.remove(key);
    _rebuild();
    return true;
  }

  /// Return the value associated with [key], or `null` when absent.
  V? search(K key) {
    final leaf = _findLeaf(key);
    final index = leaf.findKeyIndex(key, _compare);
    return index < leaf.keys.length && _compare(leaf.keys[index], key) == 0
        ? leaf.values[index]
        : null;
  }

  /// Whether [key] is present, including when its value is `null`.
  bool contains(K key) {
    final leaf = _findLeaf(key);
    final index = leaf.findKeyIndex(key, _compare);
    return index < leaf.keys.length && _compare(leaf.keys[index], key) == 0;
  }

  /// Return the smallest key, throwing [StateError] when empty.
  K minKey() {
    if (isEmpty) throw StateError('Tree is empty.');
    return _firstLeaf.keys.first;
  }

  /// Return the largest key, throwing [StateError] when empty.
  K maxKey() {
    if (isEmpty) throw StateError('Tree is empty.');
    var node = _root;
    while (node is _BPlusInternalNode<K, V>) {
      node = node.children.last;
    }
    return (node as _BPlusLeafNode<K, V>).keys.last;
  }

  /// Return entries in the inclusive key range [low] through [high].
  ///
  /// The first leaf costs O(log n) to find; the remainder follows linked leaves.
  List<BPlusTreeEntry<K, V>> rangeScan(K low, K high) {
    if (_compare(low, high) > 0) {
      throw ArgumentError('Low key must be less than or equal to high key.');
    }

    final result = <BPlusTreeEntry<K, V>>[];
    _BPlusLeafNode<K, V>? leaf = _findLeaf(low);
    while (leaf != null) {
      for (var index = 0; index < leaf.keys.length; index++) {
        final key = leaf.keys[index];
        if (_compare(key, high) > 0) return result;
        if (_compare(key, low) >= 0) {
          result.add((key, leaf.values[index]));
        }
      }
      leaf = leaf.next;
    }
    return result;
  }

  /// Alias for [rangeScan], shared with the DT11 B-tree API.
  List<BPlusTreeEntry<K, V>> rangeQuery(K low, K high) => rangeScan(low, high);

  /// Return every entry in ascending order by walking the linked leaves.
  List<BPlusTreeEntry<K, V>> fullScan() {
    final result = <BPlusTreeEntry<K, V>>[];
    _BPlusLeafNode<K, V>? leaf = _firstLeaf;
    while (leaf != null) {
      for (var index = 0; index < leaf.keys.length; index++) {
        result.add((leaf.keys[index], leaf.values[index]));
      }
      leaf = leaf.next;
    }
    return result;
  }

  /// Alias for [fullScan], shared with the DT11 B-tree API.
  List<BPlusTreeEntry<K, V>> inorder() => fullScan();

  /// A snapshot iterable of all entries in sorted order.
  Iterable<BPlusTreeEntry<K, V>> get entries => fullScan();

  /// Verify degree bounds, separators, leaf depth, links, and stored entries.
  bool isValid() {
    if (isEmpty) {
      return _root is _BPlusLeafNode<K, V> &&
          identical(_root, _firstLeaf) &&
          _firstLeaf.keys.isEmpty &&
          _firstLeaf.values.isEmpty &&
          _firstLeaf.next == null;
    }

    final leafDepth = <int?>[null];
    if (!_validateNode(_root, isRoot: true, depth: 0, leafDepth: leafDepth)) {
      return false;
    }

    final scan = <BPlusTreeEntry<K, V>>[];
    final visited = <_BPlusLeafNode<K, V>>{};
    _BPlusLeafNode<K, V>? leaf = _firstLeaf;
    K? previousKey;
    var hasPrevious = false;
    while (leaf != null) {
      if (!visited.add(leaf)) return false;
      for (var index = 0; index < leaf.keys.length; index++) {
        final key = leaf.keys[index];
        if (hasPrevious && _compare(key, previousKey as K) <= 0) return false;
        scan.add((key, leaf.values[index]));
        previousKey = key;
        hasPrevious = true;
      }
      leaf = leaf.next;
    }
    if (scan.length != count) return false;

    final expected = _entries.entries.toList(growable: false);
    for (var index = 0; index < expected.length; index++) {
      final actual = scan[index];
      if (_compare(actual.$1, expected[index].key) != 0 ||
          actual.$2 != expected[index].value ||
          !contains(actual.$1) ||
          search(actual.$1) != actual.$2) {
        return false;
      }
    }
    return true;
  }

  void _rebuild() {
    if (_entries.isEmpty) {
      final leaf = _BPlusLeafNode<K, V>();
      _root = leaf;
      _firstLeaf = leaf;
      return;
    }

    final pairs = _entries.entries.toList(growable: false);
    final leaves = <_BPlusNode<K, V>>[];
    _BPlusLeafNode<K, V>? previous;
    var offset = 0;
    for (final groupSize in _partitionSizes(
      pairs.length,
      minimumDegree - 1,
      _maxKeys,
    )) {
      final leaf = _BPlusLeafNode<K, V>();
      for (var index = offset; index < offset + groupSize; index++) {
        leaf.keys.add(pairs[index].key);
        leaf.values.add(pairs[index].value);
      }
      if (previous == null) {
        _firstLeaf = leaf;
      } else {
        previous.next = leaf;
      }
      previous = leaf;
      leaves.add(leaf);
      offset += groupSize;
    }
    _root = _buildLevel(leaves);
  }

  _BPlusNode<K, V> _buildLevel(List<_BPlusNode<K, V>> children) {
    if (children.length == 1) return children.first;
    if (children.length <= _maxChildren) return _buildInternal(children);

    final parents = <_BPlusNode<K, V>>[];
    var offset = 0;
    for (final groupSize in _partitionSizes(
      children.length,
      minimumDegree,
      _maxChildren,
    )) {
      parents.add(_buildInternal(children.sublist(offset, offset + groupSize)));
      offset += groupSize;
    }
    return _buildLevel(parents);
  }

  _BPlusInternalNode<K, V> _buildInternal(List<_BPlusNode<K, V>> children) {
    final node = _BPlusInternalNode<K, V>()..children.addAll(children);
    for (var index = 1; index < children.length; index++) {
      node.keys.add(_firstKey(children[index]));
    }
    return node;
  }

  _BPlusLeafNode<K, V> _findLeaf(K key) {
    var node = _root;
    while (node is _BPlusInternalNode<K, V>) {
      var index = 0;
      while (index < node.keys.length && _compare(key, node.keys[index]) >= 0) {
        index++;
      }
      node = node.children[index];
    }
    return node as _BPlusLeafNode<K, V>;
  }

  K _firstKey(_BPlusNode<K, V> node) {
    while (node is _BPlusInternalNode<K, V>) {
      node = node.children.first;
    }
    return (node as _BPlusLeafNode<K, V>).keys.first;
  }

  List<int> _partitionSizes(int itemCount, int minSize, int maxSize) {
    if (itemCount <= maxSize) return <int>[itemCount];
    final groupCount = (itemCount + maxSize - 1) ~/ maxSize;
    final baseSize = itemCount ~/ groupCount;
    final remainder = itemCount % groupCount;
    final sizes = <int>[];
    for (var index = 0; index < groupCount; index++) {
      final size = baseSize + (index < remainder ? 1 : 0);
      if (size < minSize || size > maxSize) {
        throw StateError('Unable to partition B+ tree nodes.');
      }
      sizes.add(size);
    }
    return sizes;
  }

  bool _validateNode(
    _BPlusNode<K, V> node, {
    required bool isRoot,
    required int depth,
    required List<int?> leafDepth,
  }) {
    if (node.keys.length > _maxKeys) return false;
    if (!isRoot && node.keys.length < minimumDegree - 1) return false;
    for (var index = 1; index < node.keys.length; index++) {
      if (_compare(node.keys[index], node.keys[index - 1]) <= 0) return false;
    }

    if (node is _BPlusLeafNode<K, V>) {
      if (node.values.length != node.keys.length) return false;
      leafDepth[0] ??= depth;
      return leafDepth[0] == depth;
    }

    final internal = node as _BPlusInternalNode<K, V>;
    if (internal.children.length != internal.keys.length + 1) return false;
    if (isRoot && internal.keys.isEmpty) return false;
    for (var index = 0; index < internal.keys.length; index++) {
      if (_compare(
            internal.keys[index],
            _firstKey(internal.children[index + 1]),
          ) !=
          0) {
        return false;
      }
    }
    for (final child in internal.children) {
      if (!_validateNode(
        child,
        isRoot: false,
        depth: depth + 1,
        leafDepth: leafDepth,
      )) {
        return false;
      }
    }
    return true;
  }

  @override
  String toString() =>
      'BPlusTree(t=$minimumDegree, size=$count, height=$height)';
}
