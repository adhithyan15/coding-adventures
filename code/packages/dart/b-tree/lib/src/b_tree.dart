/// A self-balancing, multi-way search tree (DT11) in pure Dart.
///
/// A B-tree node stores several sorted key-value pairs and, for internal
/// nodes, one more child than key. The [minimumDegree] controls node width:
/// every non-root node has between `t - 1` and `2t - 1` keys. Splitting full
/// children on descent and repairing minimally-filled children before deletion
/// keep every leaf at the same depth.
library b_tree;

/// A sorted key-value entry returned by traversal and range operations.
typedef BTreeEntry<K, V> = (K, V);

Comparator<T> _naturalOrder<T>() => (left, right) =>
    Comparable.compare(left as Comparable, right as Comparable);

final class _BTreeNode<K, V> {
  _BTreeNode({required this.isLeaf});

  final bool isLeaf;
  final List<K> keys = <K>[];
  final List<V> values = <V>[];
  final List<_BTreeNode<K, V>> children = <_BTreeNode<K, V>>[];

  bool isFull(int minimumDegree) => keys.length == 2 * minimumDegree - 1;

  /// Return the first index whose key is greater than or equal to [key].
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

/// A generic minimum-degree B-tree mapping ordered keys to values.
///
/// Inserting an existing key updates its value without changing [count].
/// [delete] is a no-op that returns `false` when the key is absent. Traversal
/// methods return Dart 3 records in ascending key order.
final class BTree<K, V> {
  /// Create an empty B-tree with minimum degree [minimumDegree].
  ///
  /// The classic 2-3-4 tree uses the default degree of 2. Natural [Comparable]
  /// order is used unless [compare] is supplied.
  BTree([this.minimumDegree = 2, Comparator<K>? compare])
      : _compare = compare ?? _naturalOrder<K>() {
    if (minimumDegree < 2) {
      throw ArgumentError.value(
        minimumDegree,
        'minimumDegree',
        'must be at least 2',
      );
    }
  }

  /// The minimum number of children in a non-root internal node.
  final int minimumDegree;
  final Comparator<K> _compare;

  _BTreeNode<K, V> _root = _BTreeNode<K, V>(isLeaf: true);
  int _count = 0;

  /// Number of key-value pairs stored in the tree.
  int get count => _count;

  /// Whether the tree contains no keys.
  bool get isEmpty => _count == 0;

  /// Number of edges from the root to any leaf.
  int get height {
    var node = _root;
    var result = 0;
    while (!node.isLeaf) {
      node = node.children.first;
      result++;
    }
    return result;
  }

  /// Insert [key] with [value], updating the value when [key] already exists.
  void insert(K key, V value) {
    if (_root.isFull(minimumDegree)) {
      final newRoot = _BTreeNode<K, V>(isLeaf: false)..children.add(_root);
      _splitChild(newRoot, 0);
      _root = newRoot;
    }

    if (_insertNonFull(_root, key, value)) {
      _count++;
    }
  }

  /// Return the value associated with [key], or `null` when it is absent.
  V? search(K key) => _search(_root, key);

  /// Whether [key] is present, including when its stored value is `null`.
  bool contains(K key) => _contains(_root, key);

  /// Remove [key], returning whether it was present.
  ///
  /// This follows the CLRS top-down deletion algorithm. Before descending, a
  /// minimally-filled child borrows from a sibling or merges with one.
  bool delete(K key) {
    // A failed top-down deletion may still merge nodes. Avoid that surprising
    // mutation and the associated root-repair edge case for absent keys.
    if (!contains(key)) return false;

    _deleteRecursive(_root, key);
    _count--;
    if (_root.keys.isEmpty && !_root.isLeaf) {
      _root = _root.children.first;
    }
    return true;
  }

  /// Return the smallest key.
  ///
  /// Throws [StateError] when the tree is empty.
  K minKey() {
    if (isEmpty) throw StateError('Tree is empty.');
    return _minNode(_root).keys.first;
  }

  /// Return the largest key.
  ///
  /// Throws [StateError] when the tree is empty.
  K maxKey() {
    if (isEmpty) throw StateError('Tree is empty.');
    return _maxNode(_root).keys.last;
  }

  /// Return entries whose keys are in the inclusive range [low] through [high].
  List<BTreeEntry<K, V>> rangeQuery(K low, K high) {
    final result = <BTreeEntry<K, V>>[];
    if (_compare(low, high) > 0) return result;
    _collectRange(_root, low, high, result);
    return result;
  }

  /// Return every entry in ascending key order.
  List<BTreeEntry<K, V>> inorder() {
    final result = <BTreeEntry<K, V>>[];
    _collectInorder(_root, result);
    return result;
  }

  /// Verify key-count, ordering, child-count, value, and leaf-depth invariants.
  bool isValid() {
    if (_count == 0) {
      return _root.isLeaf &&
          _root.keys.isEmpty &&
          _root.values.isEmpty &&
          _root.children.isEmpty;
    }
    if (inorder().length != _count) return false;
    final leafDepth = <int?>[null];
    return _validate(
      _root,
      minKey: null,
      hasMinKey: false,
      maxKey: null,
      hasMaxKey: false,
      depth: 0,
      leafDepth: leafDepth,
      isRoot: true,
    );
  }

  V? _search(_BTreeNode<K, V> node, K key) {
    final index = node.findKeyIndex(key, _compare);
    if (index < node.keys.length && _compare(node.keys[index], key) == 0) {
      return node.values[index];
    }
    return node.isLeaf ? null : _search(node.children[index], key);
  }

  bool _contains(_BTreeNode<K, V> node, K key) {
    final index = node.findKeyIndex(key, _compare);
    if (index < node.keys.length && _compare(node.keys[index], key) == 0) {
      return true;
    }
    return !node.isLeaf && _contains(node.children[index], key);
  }

  void _splitChild(_BTreeNode<K, V> parent, int childIndex) {
    final child = parent.children[childIndex];
    final right = _BTreeNode<K, V>(isLeaf: child.isLeaf);
    final middle = minimumDegree - 1;
    final medianKey = child.keys[middle];
    final medianValue = child.values[middle];

    right.keys.addAll(child.keys.sublist(middle + 1));
    right.values.addAll(child.values.sublist(middle + 1));
    if (!child.isLeaf) {
      right.children.addAll(child.children.sublist(minimumDegree));
      child.children.removeRange(minimumDegree, child.children.length);
    }

    child.keys.removeRange(middle, child.keys.length);
    child.values.removeRange(middle, child.values.length);
    parent.keys.insert(childIndex, medianKey);
    parent.values.insert(childIndex, medianValue);
    parent.children.insert(childIndex + 1, right);
  }

  bool _insertNonFull(_BTreeNode<K, V> node, K key, V value) {
    var index = node.findKeyIndex(key, _compare);
    if (index < node.keys.length && _compare(node.keys[index], key) == 0) {
      node.values[index] = value;
      return false;
    }

    if (node.isLeaf) {
      node.keys.insert(index, key);
      node.values.insert(index, value);
      return true;
    }

    if (node.children[index].isFull(minimumDegree)) {
      _splitChild(node, index);
      final comparison = _compare(key, node.keys[index]);
      if (comparison == 0) {
        node.values[index] = value;
        return false;
      }
      if (comparison > 0) index++;
    }
    return _insertNonFull(node.children[index], key, value);
  }

  void _deleteRecursive(_BTreeNode<K, V> node, K key) {
    final index = node.findKeyIndex(key, _compare);
    final found =
        index < node.keys.length && _compare(node.keys[index], key) == 0;

    if (found) {
      if (node.isLeaf) {
        node.keys.removeAt(index);
        node.values.removeAt(index);
        return;
      }

      final left = node.children[index];
      final right = node.children[index + 1];
      if (left.keys.length >= minimumDegree) {
        final predecessor = _maxNode(left);
        final predecessorKey = predecessor.keys.last;
        node.keys[index] = predecessorKey;
        node.values[index] = predecessor.values.last;
        _deleteRecursive(left, predecessorKey);
      } else if (right.keys.length >= minimumDegree) {
        final successor = _minNode(right);
        final successorKey = successor.keys.first;
        node.keys[index] = successorKey;
        node.values[index] = successor.values.first;
        _deleteRecursive(right, successorKey);
      } else {
        final merged = _mergeChildren(node, index);
        _deleteRecursive(merged, key);
      }
      return;
    }

    if (node.isLeaf) return;
    final childIndex = _ensureMinKeys(node, index);
    _deleteRecursive(node.children[childIndex], key);
  }

  _BTreeNode<K, V> _mergeChildren(_BTreeNode<K, V> parent, int leftIndex) {
    final left = parent.children[leftIndex];
    final right = parent.children[leftIndex + 1];

    left.keys.add(parent.keys.removeAt(leftIndex));
    left.values.add(parent.values.removeAt(leftIndex));
    parent.children.removeAt(leftIndex + 1);
    left.keys.addAll(right.keys);
    left.values.addAll(right.values);
    if (!left.isLeaf) left.children.addAll(right.children);
    return left;
  }

  int _ensureMinKeys(_BTreeNode<K, V> parent, int childIndex) {
    final child = parent.children[childIndex];
    if (child.keys.length >= minimumDegree) return childIndex;

    if (childIndex > 0) {
      final left = parent.children[childIndex - 1];
      if (left.keys.length >= minimumDegree) {
        child.keys.insert(0, parent.keys[childIndex - 1]);
        child.values.insert(0, parent.values[childIndex - 1]);
        parent.keys[childIndex - 1] = left.keys.removeLast();
        parent.values[childIndex - 1] = left.values.removeLast();
        if (!left.isLeaf) child.children.insert(0, left.children.removeLast());
        return childIndex;
      }
    }

    if (childIndex < parent.children.length - 1) {
      final right = parent.children[childIndex + 1];
      if (right.keys.length >= minimumDegree) {
        child.keys.add(parent.keys[childIndex]);
        child.values.add(parent.values[childIndex]);
        parent.keys[childIndex] = right.keys.removeAt(0);
        parent.values[childIndex] = right.values.removeAt(0);
        if (!right.isLeaf) child.children.add(right.children.removeAt(0));
        return childIndex;
      }
    }

    if (childIndex > 0) {
      _mergeChildren(parent, childIndex - 1);
      return childIndex - 1;
    }
    _mergeChildren(parent, childIndex);
    return childIndex;
  }

  _BTreeNode<K, V> _minNode(_BTreeNode<K, V> node) {
    while (!node.isLeaf) {
      node = node.children.first;
    }
    return node;
  }

  _BTreeNode<K, V> _maxNode(_BTreeNode<K, V> node) {
    while (!node.isLeaf) {
      node = node.children.last;
    }
    return node;
  }

  void _collectInorder(_BTreeNode<K, V> node, List<BTreeEntry<K, V>> result) {
    if (node.isLeaf) {
      for (var index = 0; index < node.keys.length; index++) {
        result.add((node.keys[index], node.values[index]));
      }
      return;
    }

    for (var index = 0; index < node.keys.length; index++) {
      _collectInorder(node.children[index], result);
      result.add((node.keys[index], node.values[index]));
    }
    _collectInorder(node.children.last, result);
  }

  void _collectRange(
    _BTreeNode<K, V> node,
    K low,
    K high,
    List<BTreeEntry<K, V>> result,
  ) {
    var index = 0;
    while (index < node.keys.length) {
      final key = node.keys[index];
      if (!node.isLeaf && _compare(low, key) < 0) {
        _collectRange(node.children[index], low, high, result);
      }
      if (_compare(key, high) > 0) return;
      if (_compare(key, low) >= 0) {
        result.add((key, node.values[index]));
      }
      index++;
    }
    if (!node.isLeaf) {
      _collectRange(node.children[index], low, high, result);
    }
  }

  bool _validate(
    _BTreeNode<K, V> node, {
    required K? minKey,
    required bool hasMinKey,
    required K? maxKey,
    required bool hasMaxKey,
    required int depth,
    required List<int?> leafDepth,
    required bool isRoot,
  }) {
    final keyCount = node.keys.length;
    if (node.values.length != keyCount) return false;
    if (keyCount > 2 * minimumDegree - 1) return false;
    if (isRoot) {
      if (_count > 0 && keyCount < 1) return false;
    } else if (keyCount < minimumDegree - 1) {
      return false;
    }

    for (var index = 0; index < keyCount; index++) {
      final key = node.keys[index];
      if (hasMinKey && _compare(key, minKey as K) <= 0) return false;
      if (hasMaxKey && _compare(key, maxKey as K) >= 0) return false;
      if (index > 0 && _compare(key, node.keys[index - 1]) <= 0) {
        return false;
      }
    }

    if (node.isLeaf) {
      if (node.children.isNotEmpty) return false;
      leafDepth[0] ??= depth;
      return leafDepth[0] == depth;
    }
    if (node.children.length != keyCount + 1) return false;

    for (var index = 0; index <= keyCount; index++) {
      final childMin = index > 0 ? node.keys[index - 1] : minKey;
      final childMax = index < keyCount ? node.keys[index] : maxKey;
      if (!_validate(
        node.children[index],
        minKey: childMin,
        hasMinKey: index > 0 || hasMinKey,
        maxKey: childMax,
        hasMaxKey: index < keyCount || hasMaxKey,
        depth: depth + 1,
        leafDepth: leafDepth,
        isRoot: false,
      )) {
        return false;
      }
    }
    return true;
  }

  @override
  String toString() => 'BTree(t=$minimumDegree, size=$count, height=$height)';
}
