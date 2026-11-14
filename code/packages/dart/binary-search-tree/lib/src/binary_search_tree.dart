/// A persistent, comparator-driven binary search tree with order statistics.
///
/// Nodes are immutable. Insertions and deletions rebuild only the search path,
/// sharing all other nodes with the previous tree. Each node caches its subtree
/// size so [BST.rank] and [BST.kthSmallest] follow one path instead of scanning.
library binary_search_tree;

/// One immutable node in a [BST].
final class BSTNode<T extends Object> {
  BSTNode._(this.value, this.left, this.right)
      : size = 1 + (left?.size ?? 0) + (right?.size ?? 0);

  final T value;
  final BSTNode<T>? left;
  final BSTNode<T>? right;
  final int size;
}

/// An immutable binary search tree.
///
/// The optional [compare] function establishes ordering and equality. Returning
/// zero means that two keys occupy the same tree position. If omitted, values
/// must implement [Comparable].
final class BST<T extends Object> {
  BST({Comparator<T>? compare})
      : _root = null,
        _compare = compare ?? _defaultCompare;

  BST._(this._root, this._compare);

  final BSTNode<T>? _root;
  final Comparator<T> _compare;

  static int _defaultCompare<T extends Object>(T a, T b) =>
      (a as Comparable<dynamic>).compareTo(b);

  /// The root node, or `null` when the tree is empty.
  BSTNode<T>? get root => _root;

  /// Build a height-balanced tree from strictly increasing [values].
  ///
  /// Strict increase is checked under the same comparator used for later
  /// operations. Empty input produces an empty tree.
  factory BST.fromSortedArray(List<T> values, {Comparator<T>? compare}) {
    final ordering = compare ?? BST._defaultCompare<T>;
    for (var i = 1; i < values.length; i++) {
      if (ordering(values[i - 1], values[i]) >= 0) {
        throw ArgumentError.value(
          values,
          'values',
          'must be strictly increasing under the comparator',
        );
      }
    }

    BSTNode<T>? build(int start, int end) {
      if (start >= end) return null;
      final middle = start + (end - start) ~/ 2;
      return BSTNode<T>._(
        values[middle],
        build(start, middle),
        build(middle + 1, end),
      );
    }

    return BST<T>._(build(0, values.length), ordering);
  }

  /// Number of values stored in this tree.
  int get size => _root?.size ?? 0;

  /// Height in edges, or `-1` for an empty tree.
  int get height {
    int visit(BSTNode<T>? node) {
      if (node == null) return -1;
      final leftHeight = visit(node.left);
      final rightHeight = visit(node.right);
      return 1 + (leftHeight > rightHeight ? leftHeight : rightHeight);
    }

    return visit(_root);
  }

  /// Return the node whose key compares equal to [value], if present.
  BSTNode<T>? search(T value) {
    var node = _root;
    while (node != null) {
      final order = _compare(value, node.value);
      if (order == 0) return node;
      node = order < 0 ? node.left : node.right;
    }
    return null;
  }

  /// Whether a key comparator-equivalent to [value] is present.
  bool contains(T value) => search(value) != null;

  /// Return a tree with [value] inserted, preserving this tree.
  ///
  /// If a comparator-equivalent key already exists, the existing value and
  /// tree are retained.
  BST<T> insert(T value) {
    BSTNode<T>? add(BSTNode<T>? node) {
      if (node == null) return BSTNode<T>._(value, null, null);
      final order = _compare(value, node.value);
      if (order == 0) return node;
      if (order < 0) {
        final left = add(node.left);
        if (identical(left, node.left)) return node;
        return BSTNode<T>._(node.value, left, node.right);
      }
      final right = add(node.right);
      if (identical(right, node.right)) return node;
      return BSTNode<T>._(node.value, node.left, right);
    }

    final root = add(_root);
    return identical(root, _root) ? this : BST<T>._(root, _compare);
  }

  /// Return a tree with [value] removed, preserving this tree.
  BST<T> delete(T value) {
    (BSTNode<T>?, T) extractMin(BSTNode<T> node) {
      if (node.left == null) return (node.right, node.value);
      final (left, value) = extractMin(node.left!);
      return (BSTNode<T>._(node.value, left, node.right), value);
    }

    (BSTNode<T>?, bool) remove(BSTNode<T>? node) {
      if (node == null) return (null, false);
      final order = _compare(value, node.value);
      if (order < 0) {
        final (left, changed) = remove(node.left);
        return changed
            ? (BSTNode<T>._(node.value, left, node.right), true)
            : (node, false);
      }
      if (order > 0) {
        final (right, changed) = remove(node.right);
        return changed
            ? (BSTNode<T>._(node.value, node.left, right), true)
            : (node, false);
      }

      if (node.left == null) return (node.right, true);
      if (node.right == null) return (node.left, true);
      final (right, successor) = extractMin(node.right!);
      return (BSTNode<T>._(successor, node.left, right), true);
    }

    final (root, changed) = remove(_root);
    return changed ? BST<T>._(root, _compare) : this;
  }

  /// Smallest key, or `null` when empty.
  T? get minValue {
    var node = _root;
    if (node == null) return null;
    while (node!.left != null) {
      node = node.left;
    }
    return node.value;
  }

  /// Largest key, or `null` when empty.
  T? get maxValue {
    var node = _root;
    if (node == null) return null;
    while (node!.right != null) {
      node = node.right;
    }
    return node.value;
  }

  /// Largest key strictly less than [value], even when [value] is absent.
  T? predecessor(T value) {
    var node = _root;
    T? best;
    while (node != null) {
      if (_compare(value, node.value) <= 0) {
        node = node.left;
      } else {
        best = node.value;
        node = node.right;
      }
    }
    return best;
  }

  /// Smallest key strictly greater than [value], even when [value] is absent.
  T? successor(T value) {
    var node = _root;
    T? best;
    while (node != null) {
      if (_compare(value, node.value) >= 0) {
        node = node.right;
      } else {
        best = node.value;
        node = node.left;
      }
    }
    return best;
  }

  /// Return the one-based [k]th key, or `null` for [k] outside `1..size`.
  T? kthSmallest(int k) {
    if (k < 1 || k > size) return null;
    var node = _root;
    var index = k;
    while (node != null) {
      final leftSize = node.left?.size ?? 0;
      if (index == leftSize + 1) return node.value;
      if (index <= leftSize) {
        node = node.left;
      } else {
        index -= leftSize + 1;
        node = node.right;
      }
    }
    return null;
  }

  /// Count keys strictly less than [value], whether or not it is present.
  int rank(T value) {
    var node = _root;
    var count = 0;
    while (node != null) {
      final order = _compare(value, node.value);
      if (order <= 0) {
        node = node.left;
      } else {
        count += (node.left?.size ?? 0) + 1;
        node = node.right;
      }
    }
    return count;
  }

  /// Values in comparator order.
  List<T> toSortedArray() {
    final values = <T>[];
    void visit(BSTNode<T>? node) {
      if (node == null) return;
      visit(node.left);
      values.add(node.value);
      visit(node.right);
    }

    visit(_root);
    return values;
  }

  /// Whether every node satisfies strict ordering bounds and size metadata.
  bool get isValid {
    ({bool valid, int size}) check(
      BSTNode<T>? node,
      T? lower,
      bool hasLower,
      T? upper,
      bool hasUpper,
    ) {
      if (node == null) return (valid: true, size: 0);
      if ((hasLower && _compare(node.value, lower as T) <= 0) ||
          (hasUpper && _compare(node.value, upper as T) >= 0)) {
        return (valid: false, size: 0);
      }
      final left = check(node.left, lower, hasLower, node.value, true);
      final right = check(node.right, node.value, true, upper, hasUpper);
      final expectedSize = left.size + right.size + 1;
      return (
        valid: left.valid && right.valid && node.size == expectedSize,
        size: expectedSize,
      );
    }

    return check(_root, null, false, null, false).valid;
  }
}
