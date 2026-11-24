/// A rooted tree backed by a directed graph.
///
/// The Dart lane of the Rust `tree` crate (spec DT02): [Tree] wraps the
/// in-lane [Graph], enforcing one root, one parent per node, and no cycles,
/// and provides traversals, depth/height, lowest common ancestor, subtree
/// extraction, and ASCII visualization.
///
/// Edges point parent → child. Children orderings are sorted alphabetically
/// wherever the reference sorts. Errors throw [TreeError] (never returned).
/// Display strings quote node names with plain double quotes; names needing
/// escapes render slightly differently than the reference's Rust
/// debug-format (documented, untested upstream either).
library;

import 'dart:collection';

import 'package:coding_adventures_directed_graph/directed_graph.dart';

/// A [Tree] invariant violation: unknown node, duplicate node, or root
/// removal.
sealed class TreeError implements Exception {
  const TreeError();
}

/// An operation referenced a node that is not in the tree.
final class NodeNotFound extends TreeError {
  /// The missing node.
  final String node;
  const NodeNotFound(this.node);

  @override
  bool operator ==(Object other) => other is NodeNotFound && node == other.node;

  @override
  int get hashCode => node.hashCode;

  @override
  String toString() => 'node not found in tree: "$node"';
}

/// [Tree.addChild] was given a child that already exists (a second parent
/// for one node would break the tree invariant).
final class DuplicateNode extends TreeError {
  /// The duplicated node.
  final String node;
  const DuplicateNode(this.node);

  @override
  bool operator ==(Object other) =>
      other is DuplicateNode && node == other.node;

  @override
  int get hashCode => node.hashCode;

  @override
  String toString() => 'node already exists in tree: "$node"';
}

/// [Tree.removeSubtree] was called on the root (the tree's anchor).
final class RootRemoval extends TreeError {
  const RootRemoval();

  @override
  bool operator ==(Object other) => other is RootRemoval;

  @override
  int get hashCode => 0;

  @override
  String toString() => 'cannot remove the root node';
}

/// A rooted tree: one root, one parent per node, no cycles.
///
/// Backed by a [Graph] with parent → child edges. A tree with N nodes holds
/// exactly N-1 edges.
class Tree {
  final Graph _graph = Graph();
  final String _root;

  /// Creates a tree holding only [root].
  Tree(String root) : _root = root {
    _graph.addNode(root);
  }

  /// The root node. Set at construction, never changes.
  String get root => _root;

  /// A snapshot copy of the underlying directed graph.
  ///
  /// The reference hands out a shared (`&`) borrow, which the borrow
  /// checker keeps read-only; Dart has no equivalent, so this returns a
  /// detached copy instead. Inspect it freely ([Graph.nodes],
  /// [Graph.edges], [Graph.hasCycle], [Graph.topologicalSort]); mutating
  /// it cannot affect the tree or break its invariants.
  Graph get graph {
    final copy = Graph();
    for (final node in _graph.nodes()) {
      copy.addNode(node);
    }
    for (final edge in _graph.edges()) {
      copy.addEdge(edge.from, edge.to);
    }
    return copy;
  }

  /// Adds [child] under [parent].
  ///
  /// Throws [NodeNotFound] when [parent] is absent (checked first) and
  /// [DuplicateNode] when [child] is present.
  void addChild(String parent, String child) {
    if (!_graph.hasNode(parent)) {
      throw NodeNotFound(parent);
    }
    if (_graph.hasNode(child)) {
      throw DuplicateNode(child);
    }
    // Cannot self-loop: the child is brand new, so parent != child and the
    // graph accepts the edge unconditionally.
    _graph.addEdge(parent, child);
  }

  /// Removes [node] and all its descendants (prunes the branch).
  ///
  /// Throws [NodeNotFound] when [node] is absent and [RootRemoval] for the
  /// root. Nodes drop deepest-first (reversed BFS).
  void removeSubtree(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    if (node == _root) {
      throw const RootRemoval();
    }
    final toRemove = _collectSubtreeNodes(node);
    for (final name in toRemove.reversed) {
      _graph.removeNode(name);
    }
  }

  /// All nodes under [node] (inclusive) in BFS order, children sorted.
  List<String> _collectSubtreeNodes(String node) {
    final result = <String>[];
    final queue = ListQueue<String>()..add(node);
    while (queue.isNotEmpty) {
      final current = queue.removeFirst();
      result.add(current);
      final children = List.of(_graph.successors(current))..sort();
      queue.addAll(children);
    }
    return result;
  }

  /// The parent of [node], or null for the root.
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  String? parent(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    final preds = _graph.predecessors(node);
    if (preds.isEmpty) return null;
    return preds.first;
  }

  /// The children of [node], sorted alphabetically.
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  List<String> children(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    return List.of(_graph.successors(node))..sort();
  }

  /// The siblings of [node] (same parent's other children, sorted).
  ///
  /// The root has no siblings. Throws [NodeNotFound] when absent.
  List<String> siblings(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    final parentNode = parent(node);
    if (parentNode == null) return [];
    return children(parentNode).where((c) => c != node).toList();
  }

  /// True iff [node] has no children.
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  bool isLeaf(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    return _graph.successors(node).isEmpty;
  }

  /// True iff [node] is the root.
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  bool isRoot(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    return node == _root;
  }

  /// Depth of [node]: root 0, children 1, and so on.
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  int depth(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    var d = 0;
    var current = node;
    while (current != _root) {
      current = _graph.predecessors(current).first;
      d++;
    }
    return d;
  }

  /// Height of the tree: the maximum depth (0 for a single node).
  int height() {
    var maxDepth = 0;
    final queue = ListQueue<(String, int)>()..add((_root, 0));
    while (queue.isNotEmpty) {
      final (current, d) = queue.removeFirst();
      if (d > maxDepth) maxDepth = d;
      for (final child in _graph.successors(current)) {
        queue.add((child, d + 1));
      }
    }
    return maxDepth;
  }

  /// Total number of nodes.
  int size() => _graph.size;

  /// All nodes, sorted alphabetically.
  List<String> nodes() => List.of(_graph.nodes())..sort();

  /// All leaf nodes, sorted alphabetically.
  List<String> leaves() {
    final result =
        _graph.nodes().where((n) => _graph.successors(n).isEmpty).toList();
    result.sort();
    return result;
  }

  /// True iff [node] is in the tree.
  bool hasNode(String node) => _graph.hasNode(node);

  /// Nodes in preorder: each parent before its children.
  ///
  /// Explicit stack; children push in reverse sorted order so the smallest
  /// pops first.
  List<String> preorder() {
    final result = <String>[];
    final stack = <String>[_root];
    while (stack.isNotEmpty) {
      final node = stack.removeLast();
      result.add(node);
      final children = List.of(_graph.successors(node))..sort();
      stack.addAll(children.reversed);
    }
    return result;
  }

  /// Nodes in postorder: children before their parent, sorted.
  ///
  /// Iterative (explicit stack with visited flags), so arbitrarily deep
  /// trees cannot overflow the call stack — unlike the reference's
  /// recursion, which this matches output-for-output.
  List<String> postorder() {
    final result = <String>[];
    // Each entry is (node, childrenEmitted).
    final stack = <(String, bool)>[(_root, false)];
    while (stack.isNotEmpty) {
      final (node, emitted) = stack.removeLast();
      if (emitted) {
        result.add(node);
        continue;
      }
      stack.add((node, true));
      final children = List.of(_graph.successors(node))..sort();
      for (final child in children.reversed) {
        stack.add((child, false));
      }
    }
    return result;
  }

  /// Nodes in level order (breadth-first), children sorted.
  List<String> levelOrder() {
    final result = <String>[];
    final queue = ListQueue<String>()..add(_root);
    while (queue.isNotEmpty) {
      final node = queue.removeFirst();
      result.add(node);
      final children = List.of(_graph.successors(node))..sort();
      queue.addAll(children);
    }
    return result;
  }

  /// Path from the root to [node] (inclusive).
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  List<String> pathTo(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    final path = <String>[];
    String? current = node;
    while (current != null) {
      path.add(current);
      current = parent(current);
    }
    return path.reversed.toList();
  }

  /// Lowest common ancestor of [a] and [b]: the deepest shared ancestor.
  ///
  /// Throws [NodeNotFound] when either is absent ([a] is checked first).
  String lca(String a, String b) {
    if (!_graph.hasNode(a)) {
      throw NodeNotFound(a);
    }
    if (!_graph.hasNode(b)) {
      throw NodeNotFound(b);
    }
    final pathA = pathTo(a);
    final pathB = pathTo(b);
    var lcaNode = _root;
    final shared = pathA.length < pathB.length ? pathA.length : pathB.length;
    for (var i = 0; i < shared; i++) {
      if (pathA[i] == pathB[i]) {
        lcaNode = pathA[i];
      } else {
        break;
      }
    }
    return lcaNode;
  }

  /// The subtree rooted at [node], as a new tree (this tree unchanged).
  ///
  /// Throws [NodeNotFound] when [node] is absent.
  Tree subtree(String node) {
    if (!_graph.hasNode(node)) {
      throw NodeNotFound(node);
    }
    final grown = Tree(node);
    final queue = ListQueue<String>()..add(node);
    while (queue.isNotEmpty) {
      final current = queue.removeFirst();
      final children = List.of(_graph.successors(current))..sort();
      for (final child in children) {
        // Cannot fail: fresh tree, each child added exactly once.
        grown.addChild(current, child);
        queue.add(child);
      }
    }
    return grown;
  }

  /// Renders the tree as ASCII art:
  ///
  /// ```text
  /// Program
  /// ├── Assignment
  /// │   ├── BinaryOp
  /// │   └── Name
  /// └── Print
  /// ```
  /// Iterative over an explicit frame stack, so arbitrarily deep trees
  /// cannot overflow the call stack — unlike the reference's recursion,
  /// which this matches output-for-output.
  String toAscii() {
    final lines = <String>[];
    final stack = [_AsciiFrame(_root, '', '')];
    while (stack.isNotEmpty) {
      final frame = stack.last;
      if (frame.children == null) {
        lines.add('${frame.prefix}${frame.node}');
        frame.children = List.of(_graph.successors(frame.node))..sort();
      }
      final children = frame.children!;
      if (frame.nextChild < children.length) {
        final child = children[frame.nextChild];
        frame.nextChild++;
        final last = frame.nextChild == children.length;
        stack.add(last
            ? _AsciiFrame(
                child, '${frame.childPrefix}└── ', '${frame.childPrefix}    ')
            : _AsciiFrame(
                child, '${frame.childPrefix}├── ', '${frame.childPrefix}│   '));
      } else {
        stack.removeLast();
      }
    }
    return lines.join('\n');
  }

  @override
  String toString() => 'Tree(root="$_root", size=${size()})';
}

/// One pending node in the iterative [Tree.toAscii] walk: its line
/// prefixes plus how many children have been emitted so far.
class _AsciiFrame {
  final String node;
  final String prefix;
  final String childPrefix;
  List<String>? children;
  int nextChild = 0;

  _AsciiFrame(this.node, this.prefix, this.childPrefix);
}
