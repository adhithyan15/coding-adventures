import 'dart:collection';

/// The largest positional projection accepted or emitted by this package.
///
/// A positional array can grow exponentially for a skewed tree. Keeping this
/// limit in the data-structure layer makes hostile shapes fail before they can
/// allocate an unbounded array.
const int maxBinaryTreeSlots = 1048575;

/// Stable machine-readable failures for construction and identity queries.
enum BinaryTreeErrorCode {
  tooManySlots,
  orphanSlot,
  sharedNode,
  cycle,
  positionalLimit,
  nodeNotFound,
}

/// A predictable error whose [code] is stable across message improvements.
final class BinaryTreeException implements Exception {
  const BinaryTreeException(this.code, this.message);

  final BinaryTreeErrorCode code;
  final String message;

  @override
  String toString() => 'BinaryTreeException(${code.name}): $message';
}

/// One position in a level-order projection.
///
/// A tagged slot is necessary because `null` may be a legitimate payload. A
/// missing node and a present node whose value is null are different states.
sealed class BinaryTreeSlot<T> {
  const BinaryTreeSlot._();

  const factory BinaryTreeSlot.missing() = MissingBinaryTreeSlot<T>;
  const factory BinaryTreeSlot.value(T value) = ValueBinaryTreeSlot<T>;

  bool get isMissing;
}

final class MissingBinaryTreeSlot<T> extends BinaryTreeSlot<T> {
  const MissingBinaryTreeSlot() : super._();

  @override
  bool get isMissing => true;

  @override
  bool operator ==(Object other) => other is MissingBinaryTreeSlot<T>;

  @override
  int get hashCode => Object.hash(MissingBinaryTreeSlot, T);

  @override
  String toString() => 'BinaryTreeSlot<$T>.missing()';
}

final class ValueBinaryTreeSlot<T> extends BinaryTreeSlot<T> {
  const ValueBinaryTreeSlot(this.value) : super._();

  final T value;

  @override
  bool get isMissing => false;

  @override
  bool operator ==(Object other) =>
      other is ValueBinaryTreeSlot<T> && other.value == value;

  @override
  int get hashCode => Object.hash(ValueBinaryTreeSlot, value);

  @override
  String toString() => 'BinaryTreeSlot<$T>.value($value)';
}

/// An immutable node. Node identity, not payload equality, names a position.
///
/// Payloads are retained rather than deep-copied: ownership is deliberately
/// shallow so callers can store any Dart object without a cloning protocol.
final class BinaryTreeNode<T> {
  const BinaryTreeNode(this.value, {this.left, this.right});

  final T value;
  final BinaryTreeNode<T>? left;
  final BinaryTreeNode<T>? right;
}

/// An immutable, validated binary tree.
///
/// Construction validates the complete supplied graph before publishing the
/// tree, so every failure is atomic and every later operation can be iterative.
final class BinaryTree<T> {
  BinaryTree._(this.root, this._size);

  /// Construct an empty tree.
  factory BinaryTree.empty() => BinaryTree<T>._(null, 0);

  /// Validate and adopt an immutable node graph.
  factory BinaryTree.fromRoot(BinaryTreeNode<T>? root) {
    if (root == null) return BinaryTree<T>.empty();
    final size = _validateGraph(root);
    return BinaryTree<T>._(root, size);
  }

  /// Build from positional level-order slots.
  ///
  /// Trailing missing slots are ignored. A present slot whose parent is
  /// missing is rejected instead of being silently discarded.
  factory BinaryTree.fromLevelOrder(List<BinaryTreeSlot<T>> input) {
    if (input.length > maxBinaryTreeSlots) {
      throw const BinaryTreeException(
        BinaryTreeErrorCode.tooManySlots,
        'level-order input exceeds 1048575 slots',
      );
    }

    var length = input.length;
    while (length > 0 && input[length - 1].isMissing) {
      length--;
    }
    if (length == 0) return BinaryTree<T>.empty();

    if (input[0].isMissing) {
      throw const BinaryTreeException(
        BinaryTreeErrorCode.orphanSlot,
        'a non-empty projection requires a root value',
      );
    }
    for (var index = 1; index < length; index++) {
      if (!input[index].isMissing && input[(index - 1) ~/ 2].isMissing) {
        throw BinaryTreeException(
          BinaryTreeErrorCode.orphanSlot,
          'slot $index is present while its parent is missing',
        );
      }
    }

    final nodes = List<BinaryTreeNode<T>?>.filled(length, null);
    var count = 0;
    for (var index = length - 1; index >= 0; index--) {
      final slot = input[index];
      if (slot case ValueBinaryTreeSlot<T>(:final value)) {
        final leftIndex = index * 2 + 1;
        final rightIndex = leftIndex + 1;
        nodes[index] = BinaryTreeNode<T>(
          value,
          left: leftIndex < length ? nodes[leftIndex] : null,
          right: rightIndex < length ? nodes[rightIndex] : null,
        );
        count++;
      }
    }
    return BinaryTree<T>._(nodes[0], count);
  }

  final BinaryTreeNode<T>? root;
  final int _size;

  int get size => _size;
  bool get isEmpty => root == null;

  /// Whether [node] is one of this tree's exact node objects.
  bool contains(BinaryTreeNode<T> node) {
    for (final current in _nodesPreorder()) {
      if (identical(current, node)) return true;
    }
    return false;
  }

  BinaryTreeNode<T>? leftChild(BinaryTreeNode<T> node) {
    _requireMember(node);
    return node.left;
  }

  BinaryTreeNode<T>? rightChild(BinaryTreeNode<T> node) {
    _requireMember(node);
    return node.right;
  }

  int depthOf(BinaryTreeNode<T> node) {
    final root = this.root;
    if (root == null) return _missingNode();
    final queue = Queue<(BinaryTreeNode<T>, int)>()..add((root, 0));
    while (queue.isNotEmpty) {
      final (current, depth) = queue.removeFirst();
      if (identical(current, node)) return depth;
      if (current.left case final child?) queue.add((child, depth + 1));
      if (current.right case final child?) queue.add((child, depth + 1));
    }
    return _missingNode();
  }

  int get height {
    final root = this.root;
    if (root == null) return -1;
    var maximum = 0;
    final stack = <(BinaryTreeNode<T>, int)>[(root, 0)];
    while (stack.isNotEmpty) {
      final (node, depth) = stack.removeLast();
      if (depth > maximum) maximum = depth;
      if (node.left case final child?) stack.add((child, depth + 1));
      if (node.right case final child?) stack.add((child, depth + 1));
    }
    return maximum;
  }

  bool get isFull {
    for (final node in _nodesPreorder()) {
      if ((node.left == null) != (node.right == null)) return false;
    }
    return true;
  }

  bool get isComplete {
    final root = this.root;
    if (root == null) return true;
    var sawGap = false;
    final queue = Queue<BinaryTreeNode<T>?>()..add(root);
    while (queue.isNotEmpty) {
      final node = queue.removeFirst();
      if (node == null) {
        sawGap = true;
      } else {
        if (sawGap) return false;
        queue.add(node.left);
        queue.add(node.right);
      }
    }
    return true;
  }

  bool get isPerfect {
    final root = this.root;
    if (root == null) return true;
    int? leafDepth;
    final stack = <(BinaryTreeNode<T>, int)>[(root, 0)];
    while (stack.isNotEmpty) {
      final (node, depth) = stack.removeLast();
      final left = node.left;
      final right = node.right;
      if (left == null && right == null) {
        leafDepth ??= depth;
        if (leafDepth != depth) return false;
      } else if (left == null || right == null) {
        return false;
      } else {
        stack.add((left, depth + 1));
        stack.add((right, depth + 1));
      }
    }
    return true;
  }

  List<T> inorder() {
    final output = <T>[];
    final stack = <BinaryTreeNode<T>>[];
    var current = root;
    while (current != null || stack.isNotEmpty) {
      while (current != null) {
        stack.add(current);
        current = current.left;
      }
      current = stack.removeLast();
      output.add(current.value);
      current = current.right;
    }
    return output;
  }

  List<T> preorder() => [for (final node in _nodesPreorder()) node.value];

  List<T> postorder() {
    final root = this.root;
    if (root == null) return <T>[];
    final output = <T>[];
    final stack = <(BinaryTreeNode<T>, bool)>[(root, false)];
    while (stack.isNotEmpty) {
      final (node, expanded) = stack.removeLast();
      if (expanded) {
        output.add(node.value);
      } else {
        stack.add((node, true));
        if (node.right case final child?) stack.add((child, false));
        if (node.left case final child?) stack.add((child, false));
      }
    }
    return output;
  }

  List<T> levelOrder() {
    final root = this.root;
    if (root == null) return <T>[];
    final output = <T>[];
    final queue = Queue<BinaryTreeNode<T>>()..add(root);
    while (queue.isNotEmpty) {
      final node = queue.removeFirst();
      output.add(node.value);
      if (node.left case final child?) queue.add(child);
      if (node.right case final child?) queue.add(child);
    }
    return output;
  }

  /// Return a fresh canonical positional projection.
  List<BinaryTreeSlot<T>> toArray() {
    final root = this.root;
    if (root == null) return <BinaryTreeSlot<T>>[];
    final indexed = <(BinaryTreeNode<T>, int)>[(root, 0)];
    var maximum = 0;
    for (var cursor = 0; cursor < indexed.length; cursor++) {
      final (node, index) = indexed[cursor];
      if (index > maximum) maximum = index;
      if (node.left case final child?) indexed.add((child, index * 2 + 1));
      if (node.right case final child?) indexed.add((child, index * 2 + 2));
    }
    final output = List<BinaryTreeSlot<T>>.filled(
      maximum + 1,
      BinaryTreeSlot<T>.missing(),
      growable: false,
    );
    for (final (node, index) in indexed) {
      output[index] = BinaryTreeSlot<T>.value(node.value);
    }
    return output;
  }

  Iterable<BinaryTreeNode<T>> _nodesPreorder() sync* {
    final root = this.root;
    if (root == null) return;
    final stack = <BinaryTreeNode<T>>[root];
    while (stack.isNotEmpty) {
      final node = stack.removeLast();
      yield node;
      if (node.right case final child?) stack.add(child);
      if (node.left case final child?) stack.add(child);
    }
  }

  void _requireMember(BinaryTreeNode<T> node) {
    if (!contains(node)) _missingNode();
  }

  Never _missingNode() => throw const BinaryTreeException(
        BinaryTreeErrorCode.nodeNotFound,
        'node identity is not present in this tree',
      );

  static int _validateGraph<T>(BinaryTreeNode<T> root) {
    final discovered = HashSet<BinaryTreeNode<T>>.identity();
    final active = HashSet<BinaryTreeNode<T>>.identity();
    final stack = <(BinaryTreeNode<T>, int, bool)>[(root, 0, false)];
    while (stack.isNotEmpty) {
      final (node, index, leaving) = stack.removeLast();
      if (leaving) {
        active.remove(node);
        continue;
      }
      if (active.contains(node)) {
        throw const BinaryTreeException(
          BinaryTreeErrorCode.cycle,
          'node graph contains a cycle',
        );
      }
      if (!discovered.add(node)) {
        throw const BinaryTreeException(
          BinaryTreeErrorCode.sharedNode,
          'one node identity appears at more than one position',
        );
      }
      if (index >= maxBinaryTreeSlots) {
        throw const BinaryTreeException(
          BinaryTreeErrorCode.positionalLimit,
          'tree position exceeds the 1048575-slot projection limit',
        );
      }
      active.add(node);
      stack.add((node, index, true));
      if (node.right case final child?) {
        stack.add((child, index * 2 + 2, false));
      }
      if (node.left case final child?) {
        stack.add((child, index * 2 + 1, false));
      }
    }
    return discovered.length;
  }
}
