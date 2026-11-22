/// AVL self-balancing binary search tree: the Dart lane of DT08.
///
/// An AVL tree keeps every node's left/right subtree heights within one of
/// each other, so search, insert, and delete all cost O(log n). Insertions
/// and deletions rebalance with single and double rotations on the way back
/// up; each node caches its height and subtree size for order statistics.
///
/// This lane mirrors the reference API with immutable updates: [AvlTree]
/// methods return new trees, so every intermediate version stays usable.
///
/// ```dart
/// import 'package:coding_adventures_avl_tree/coding_adventures_avl_tree.dart';
///
/// void main() {
///   final tree = AvlTree<int>.empty().insert(3).insert(1).insert(2);
///   print(tree.toSortedArray()); // [1, 2, 3]
///   print(tree.isValidAvl()); // true
/// }
/// ```
library avl_tree;

// ─── Nodes & Trees ──────────────────────────────────────────────────────────

/// One AVL node: a value plus optional children and cached metadata.
///
/// `height` counts edges on the longest downward path (leaf = 0); `size`
/// counts the whole subtree including this node.
///
/// Fields are private with read-only getters: external holders of a node
/// (via [AvlTree.root] or [AvlTree.search]) can inspect but never mutate,
/// which is what makes the immutable tree API sound. Only this library
/// links or rebalances nodes.
class AvlNode<T> {
  T _value;
  AvlNode<T>? _left;
  AvlNode<T>? _right;
  int _height;
  int _size;

  /// Creates a leaf node holding [value].
  AvlNode(this._value)
      : _left = null,
        _right = null,
        _height = 0,
        _size = 1;

  AvlNode._with(this._value, this._left, this._right, this._height, this._size);

  /// Stored value.
  T get value => _value;

  /// Left subtree (smaller values).
  AvlNode<T>? get left => _left;

  /// Right subtree (larger values).
  AvlNode<T>? get right => _right;

  /// Cached height (leaf = 0).
  int get height => _height;

  /// Cached subtree size.
  int get size => _size;

  /// Shallow copy sharing both child subtrees (path-copying primitive).
  AvlNode<T> _shallowCopy() =>
      AvlNode<T>._with(_value, _left, _right, _height, _size);

  @override
  String toString() => 'AvlNode($value)';
}

/// An immutable AVL tree over ordered values of type [T].
class AvlTree<T extends Comparable<dynamic>> {
  final AvlNode<T>? _root;

  const AvlTree._(this._root);

  /// Creates an empty tree.
  factory AvlTree.empty() => AvlTree._(null);

  /// The root node, or null when empty.
  AvlNode<T>? root() => _root;

  /// Returns a tree with [value] inserted (no-op if already present).
  ///
  /// The receiver is never modified: only nodes on the update path are
  /// cloned (path copying), unchanged subtrees are shared, and every
  /// intermediate version stays usable.
  AvlTree<T> insert(T value) => AvlTree._(_insertCow(_root, value));

  /// Returns a tree with [value] removed (no-op if absent).
  ///
  /// Like [insert], the receiver is preserved via path copying.
  AvlTree<T> delete(T value) => AvlTree._(_deleteCow(_root, value));

  /// Finds the node holding [value], or null.
  AvlNode<T>? search(T value) => avlSearch(_root, value);

  /// True when [value] is present.
  bool contains(T value) => search(value) != null;

  /// Smallest value, or null when empty.
  T? minValue() => _bstMinValue(_root);

  /// Largest value, or null when empty.
  T? maxValue() => _bstMaxValue(_root);

  /// Greatest value strictly below [value], or null.
  T? predecessor(T value) => _bstPredecessor(_root, value);

  /// Smallest value strictly above [value], or null.
  T? successor(T value) => _bstSuccessor(_root, value);

  /// 1-based k-th smallest value, or null when out of range.
  T? kthSmallest(int k) => _bstKthSmallest(_root, k);

  /// Count of values strictly below [value] (0-based rank).
  int rank(T value) => _bstRank(_root, value);

  /// All values in sorted order.
  List<T> toSortedArray() {
    final out = <T>[];
    _inOrder(_root, out);
    return out;
  }

  /// True when keys satisfy strict BST ordering.
  bool isValidBst() => _bstIsValid(_root);

  /// Balance factor of [node]: left height minus right height.
  int balanceFactor(AvlNode<T> node) =>
      avlHeight(node.left) - avlHeight(node.right);

  /// True when BST-ordered, heights/sizes exact, and every balance in [-1, 1].
  bool isValidAvl() => isValidAvlRoot(_root);

  /// Tree height in edges (-1 when empty).
  int height() => avlHeight(_root);

  /// Number of values stored.
  int size() => avlSize(_root);
}

// ─── Search, Insert, Delete ─────────────────────────────────────────────────

/// Path-copying insert: clones only nodes on the update path.
///
/// Unchanged subtrees are shared with the old version, which is safe because
/// nodes are externally immutable — no holder can mutate a shared subtree.
/// Total garbage per insert is O(log n), not O(n).
AvlNode<T>? _insertCow<T extends Comparable<dynamic>>(
    AvlNode<T>? node, T value) {
  if (node == null) return AvlNode(value);
  final copy = node._shallowCopy();
  final cmp = value.compareTo(copy._value);
  if (cmp < 0) {
    copy._left = _insertCow(node._left, value);
  } else if (cmp > 0) {
    copy._right = _insertCow(node._right, value);
  } else {
    return node;
  }
  _updateMetadata(copy);
  return _rebalanceCow(copy);
}

/// Path-copying delete: clones only nodes on the update path.
AvlNode<T>? _deleteCow<T extends Comparable<dynamic>>(
    AvlNode<T>? node, T value) {
  if (node == null) return null;
  final copy = node._shallowCopy();
  final cmp = value.compareTo(copy._value);
  if (cmp < 0) {
    copy._left = _deleteCow(node._left, value);
  } else if (cmp > 0) {
    copy._right = _deleteCow(node._right, value);
  } else {
    if (copy._left == null) return copy._right;
    if (copy._right == null) return copy._left;
    final extracted = _extractMinCow(copy._right!);
    copy._value = extracted.minValue;
    copy._right = extracted.rest;
  }
  _updateMetadata(copy);
  return _rebalanceCow(copy);
}

/// Path-copying minimum extraction for [_deleteCow].
({AvlNode<T>? rest, T minValue}) _extractMinCow<T extends Comparable<dynamic>>(
    AvlNode<T> node) {
  if (node._left == null) {
    return (rest: node._right, minValue: node._value);
  }
  final copy = node._shallowCopy();
  final extracted = _extractMinCow(node._left!);
  copy._left = extracted.rest;
  _updateMetadata(copy);
  return (rest: _rebalanceCow(copy), minValue: extracted.minValue);
}

/// Rebalance over an exclusively-owned [node] (fresh path copy).
///
/// Zig-zag rotations take possibly-shared children and clone everything
/// they write, so shared subtrees are only ever read, never relinked.
AvlNode<T> _rebalanceCow<T extends Comparable<dynamic>>(AvlNode<T> node) {
  final bf = balanceFactorOf(node);
  if (bf > 1) {
    final left = node._left;
    if (left != null && balanceFactorOf(left) < 0) {
      node._left = _rotateLeftCow(left);
    }
    return _rotateRightCow(node);
  }
  if (bf < -1) {
    final right = node._right;
    if (right != null && balanceFactorOf(right) > 0) {
      node._right = _rotateRightCow(right);
    }
    return _rotateLeftCow(node);
  }
  return node;
}

/// Left rotation cloning everything it writes.
///
/// Safe on shared input: both participants are shallow-copied first, so the
/// old version's nodes are only read, never linked or re-metadata'd.
AvlNode<T> _rotateLeftCow<T extends Comparable<dynamic>>(AvlNode<T> root) {
  final right = root._right;
  // Defensive: bf < -1 proves a right child exists, so this never fires.
  if (right == null) return root; // coverage:ignore-line
  final newRoot = right._shallowCopy();
  final demoted = root._shallowCopy();
  demoted._right = newRoot._left;
  _updateMetadata(demoted);
  newRoot._left = demoted;
  _updateMetadata(newRoot);
  return newRoot;
}

/// Right rotation cloning everything it writes (mirror of [_rotateLeftCow]).
AvlNode<T> _rotateRightCow<T extends Comparable<dynamic>>(AvlNode<T> root) {
  final left = root._left;
  // Defensive: bf > 1 proves a left child exists, so this never fires.
  if (left == null) return root; // coverage:ignore-line
  final newRoot = left._shallowCopy();
  final demoted = root._shallowCopy();
  demoted._left = newRoot._right;
  _updateMetadata(demoted);
  newRoot._right = demoted;
  _updateMetadata(newRoot);
  return newRoot;
}

/// Iterative BST search for [value] under [root].
AvlNode<T>? avlSearch<T extends Comparable<dynamic>>(
    AvlNode<T>? root, T value) {
  var current = root;
  while (current != null) {
    final cmp = value.compareTo(current.value);
    if (cmp < 0) {
      current = current.left;
    } else if (cmp > 0) {
      current = current.right;
    } else {
      return current;
    }
  }
  return null;
}

// ─── Rotations & Rebalancing ────────────────────────────────────────────────
//
// A left rotation lifts the right child into place (fixing right-heavy
// nodes); a right rotation mirrors it. Zig-zag cases first rotate the heavy
// child the other way. Metadata refreshes bottom-up: the demoted node first,
// then the promoted one.
//
// NOTE: unlike the reference lanes, this library exposes no mutating free
// functions (insert/delete/rotate/rebalance over bare nodes). Those would
// take ownership of nodes the caller might share with a live tree —
// corrupting it, since Dart has no borrow checker. All mutation flows
// through the path-copying [AvlTree] methods above.

/// Left height minus right height for [node].
int balanceFactorOf<T extends Comparable<dynamic>>(AvlNode<T> node) =>
    avlHeight(node.left) - avlHeight(node.right);

/// Height in edges (-1 for empty).
int avlHeight<T extends Comparable<dynamic>>(AvlNode<T>? root) =>
    root == null ? -1 : root.height;

/// Subtree size (0 for empty).
int avlSize<T extends Comparable<dynamic>>(AvlNode<T>? root) =>
    root == null ? 0 : root.size;

/// Full AVL validity: ordering, exact metadata, and balance in [-1, 1].
bool isValidAvlRoot<T extends Comparable<dynamic>>(AvlNode<T>? root) =>
    _validate<T>(root, null, null) != null;

/// Validation returning `(height, size)` when the subtree checks out.
(int, int)? _validate<T extends Comparable<dynamic>>(
    AvlNode<T>? node, T? min, T? max) {
  if (node == null) return (-1, 0);
  // The failure returns below are unreachable by construction (invalid
  // trees are unconstructible outside this library); they exist to fail
  // loudly on bugs. Only the defensive returns are excluded — the live
  // recursion and height computation stay inside measured coverage.
  if (min != null && node.value.compareTo(min) <= 0) {
    // coverage:ignore-line
    return null;
  }
  if (max != null && node.value.compareTo(max) >= 0) {
    // coverage:ignore-line
    return null;
  }
  final left = _validate(node.left, min, node.value);
  if (left == null) {
    // coverage:ignore-line
    return null;
  }
  final right = _validate(node.right, node.value, max);
  if (right == null) {
    // coverage:ignore-line
    return null;
  }
  final height = 1 + (left.$1 > right.$1 ? left.$1 : right.$1);
  if (node.height != height || // coverage:ignore-line
      node.size != 1 + left.$2 + right.$2) {
    return null;
  }
  if ((left.$1 - right.$1).abs() > 1) {
    // coverage:ignore-line
    return null;
  }
  return (height, 1 + left.$2 + right.$2);
}

void _updateMetadata<T extends Comparable<dynamic>>(AvlNode<T> node) {
  final leftH = avlHeight(node.left);
  final rightH = avlHeight(node.right);
  node._height = 1 + (leftH > rightH ? leftH : rightH);
  node._size = 1 + avlSize(node.left) + avlSize(node.right);
}

// ─── Queries ────────────────────────────────────────────────────────────────

T? _bstMinValue<T extends Comparable<dynamic>>(AvlNode<T>? root) {
  var current = root;
  while (current != null) {
    if (current.left == null) return current.value;
    current = current.left;
  }
  return null;
}

T? _bstMaxValue<T extends Comparable<dynamic>>(AvlNode<T>? root) {
  var current = root;
  while (current != null) {
    if (current.right == null) return current.value;
    current = current.right;
  }
  return null;
}

T? _bstPredecessor<T extends Comparable<dynamic>>(AvlNode<T>? root, T value) {
  var current = root;
  T? best;
  while (current != null) {
    final cmp = value.compareTo(current.value);
    if (cmp <= 0) {
      current = current.left;
    } else {
      best = current.value;
      current = current.right;
    }
  }
  return best;
}

T? _bstSuccessor<T extends Comparable<dynamic>>(AvlNode<T>? root, T value) {
  var current = root;
  T? best;
  while (current != null) {
    final cmp = value.compareTo(current.value);
    if (cmp >= 0) {
      current = current.right;
    } else {
      best = current.value;
      current = current.left;
    }
  }
  return best;
}

T? _bstKthSmallest<T extends Comparable<dynamic>>(AvlNode<T>? root, int k) {
  if (k == 0) return null;
  final node = root;
  if (node == null) return null;
  final leftSize = avlSize(node.left);
  if (k == leftSize + 1) return node.value;
  if (k <= leftSize) return _bstKthSmallest(node.left, k);
  return _bstKthSmallest(node.right, k - leftSize - 1);
}

int _bstRank<T extends Comparable<dynamic>>(AvlNode<T>? root, T value) {
  final node = root;
  if (node == null) return 0;
  final cmp = value.compareTo(node.value);
  if (cmp < 0) return _bstRank(node.left, value);
  if (cmp == 0) return avlSize(node.left);
  return avlSize(node.left) + 1 + _bstRank(node.right, value);
}

void _inOrder<T extends Comparable<dynamic>>(AvlNode<T>? root, List<T> out) {
  if (root == null) return;
  _inOrder(root.left, out);
  out.add(root.value);
  _inOrder(root.right, out);
}

bool _bstIsValid<T extends Comparable<dynamic>>(AvlNode<T>? root) =>
    _validateBst<T>(root, null, null);

bool _validateBst<T extends Comparable<dynamic>>(
    AvlNode<T>? node, T? min, T? max) {
  if (node == null) return true;
  // Unreachable by construction (same reasoning as _validate above).
  if (min != null && node.value.compareTo(min) <= 0) {
    // coverage:ignore-line
    return false;
  }
  if (max != null && node.value.compareTo(max) >= 0) {
    // coverage:ignore-line
    return false;
  }
  return _validateBst(node.left, min, node.value) &&
      _validateBst(node.right, node.value, max);
}
