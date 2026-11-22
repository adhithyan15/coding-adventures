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
class AvlNode<T> {
  /// Stored value.
  T value;

  /// Left subtree (smaller values).
  AvlNode<T>? left;

  /// Right subtree (larger values).
  AvlNode<T>? right;

  /// Cached height (leaf = 0).
  int height;

  /// Cached subtree size.
  int size;

  /// Creates a leaf node holding [value].
  AvlNode(this.value)
      : left = null,
        right = null,
        height = 0,
        size = 1;

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
  /// The receiver is never modified: the spine is cloned first, so every
  /// intermediate version stays usable.
  AvlTree<T> insert(T value) => AvlTree._(avlInsert(_clone(_root), value));

  /// Returns a tree with [value] removed (no-op if absent).
  ///
  /// Like [insert], the receiver is preserved via cloning.
  AvlTree<T> delete(T value) => AvlTree._(avlDelete(_clone(_root), value));

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

/// Deep-copies [node] with exact metadata (backs the immutable API).
AvlNode<T>? _clone<T extends Comparable<dynamic>>(AvlNode<T>? node) {
  if (node == null) return null;
  return AvlNode(node.value)
    ..left = _clone(node.left)
    ..right = _clone(node.right)
    ..height = node.height
    ..size = node.size;
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

/// Inserts [value], rebalancing on the way up. Duplicates are no-ops.
///
/// Takes ownership of [root] (mutates in place); callers clone first when
/// they must preserve the old version.
AvlNode<T>? avlInsert<T extends Comparable<dynamic>>(
    AvlNode<T>? root, T value) {
  if (root == null) return AvlNode(value);
  final cmp = value.compareTo(root.value);
  if (cmp < 0) {
    root.left = avlInsert(root.left, value);
  } else if (cmp > 0) {
    root.right = avlInsert(root.right, value);
  } else {
    return root;
  }
  _updateMetadata(root);
  return rebalance(root);
}

/// Deletes [value], rebalancing on the way up. Missing values are no-ops.
///
/// Two-child deletion swaps in the in-order successor (right-subtree
/// minimum), like the reference lanes.
AvlNode<T>? avlDelete<T extends Comparable<dynamic>>(
    AvlNode<T>? root, T value) {
  final node = root;
  if (node == null) return null;

  final cmp = value.compareTo(node.value);
  if (cmp < 0) {
    node.left = avlDelete(node.left, value);
  } else if (cmp > 0) {
    node.right = avlDelete(node.right, value);
  } else {
    final left = node.left;
    final right = node.right;
    if (left == null && right == null) return null;
    if (left != null && right == null) return left;
    if (left == null && right != null) return right;
    final extracted = _extractMin(right!);
    node.value = extracted.minValue;
    node.left = left;
    node.right = extracted.rest;
  }

  _updateMetadata(node);
  return rebalance(node);
}

// ─── Rotations & Rebalancing ────────────────────────────────────────────────
//
// A left rotation lifts the right child into place (fixing right-heavy
// nodes); a right rotation mirrors it. Zig-zag cases first rotate the heavy
// child the other way. Metadata refreshes bottom-up: the demoted node first,
// then the promoted one.

/// Left rotation around [root] (no-op without a right child).
AvlNode<T> rotateLeft<T extends Comparable<dynamic>>(AvlNode<T> root) {
  final newRoot = root.right;
  if (newRoot == null) return root;
  root.right = newRoot.left;
  _updateMetadata(root);
  newRoot.left = root;
  _updateMetadata(newRoot);
  return newRoot;
}

/// Right rotation around [root] (no-op without a left child).
AvlNode<T> rotateRight<T extends Comparable<dynamic>>(AvlNode<T> root) {
  final newRoot = root.left;
  if (newRoot == null) return root;
  root.left = newRoot.right;
  _updateMetadata(root);
  newRoot.right = root;
  _updateMetadata(newRoot);
  return newRoot;
}

/// Restores the AVL invariant at [node] (single or double rotation).
AvlNode<T> rebalance<T extends Comparable<dynamic>>(AvlNode<T> node) {
  final bf = balanceFactorOf(node);
  if (bf > 1) {
    final left = node.left;
    if (left != null && balanceFactorOf(left) < 0) {
      node.left = rotateLeft(left);
    }
    return rotateRight(node);
  }
  if (bf < -1) {
    final right = node.right;
    if (right != null && balanceFactorOf(right) > 0) {
      node.right = rotateRight(right);
    }
    return rotateLeft(node);
  }
  return node;
}

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
  if (min != null && node.value.compareTo(min) <= 0) return null;
  if (max != null && node.value.compareTo(max) >= 0) return null;
  final left = _validate(node.left, min, node.value);
  if (left == null) return null;
  final right = _validate(node.right, node.value, max);
  if (right == null) return null;
  final height = 1 + (left.$1 > right.$1 ? left.$1 : right.$1);
  if (node.height != height || node.size != 1 + left.$2 + right.$2) {
    return null;
  }
  if ((left.$1 - right.$1).abs() > 1) return null;
  return (height, 1 + left.$2 + right.$2);
}

void _updateMetadata<T extends Comparable<dynamic>>(AvlNode<T> node) {
  final leftH = avlHeight(node.left);
  final rightH = avlHeight(node.right);
  node.height = 1 + (leftH > rightH ? leftH : rightH);
  node.size = 1 + avlSize(node.left) + avlSize(node.right);
}

/// Removes and returns the minimum: `(remainingTree, minValue)`.
({AvlNode<T>? rest, T minValue}) _extractMin<T extends Comparable<dynamic>>(
    AvlNode<T> node) {
  final left = node.left;
  if (left == null) {
    return (rest: node.right, minValue: node.value);
  }
  final extracted = _extractMin(left);
  node.left = extracted.rest;
  _updateMetadata(node);
  return (rest: rebalance(node), minValue: extracted.minValue);
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
  if (min != null && node.value.compareTo(min) <= 0) return false;
  if (max != null && node.value.compareTo(max) >= 0) return false;
  return _validateBst(node.left, min, node.value) &&
      _validateBst(node.right, node.value, max);
}
