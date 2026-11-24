/// Property tests for `coding_adventures_tree`, using the in-lane `pbt`
/// package: structural invariants, traversal laws, path/depth/LCA
/// consistency, subtree/remove restoration, and exact error values — all
/// over randomly parented trees with fixed seeds.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_tree/coding_adventures_tree.dart';
import 'package:test/test.dart';

/// Builds a random tree: node `n0` is the root; each later node attaches
/// to a uniformly random earlier node (always a valid tree).
Tree _buildTree(List<int> parents) {
  final tree = Tree('n0');
  for (var i = 1; i < parents.length; i++) {
    tree.addChild('n${parents[i]}', 'n$i');
  }
  return tree;
}

/// Parent indices for nodes 1..n (each below its own index). Index 0 holds
/// a dummy (the root has no parent).
Gen<List<int>> _parents(int n) {
  if (n <= 1) return Gen.constant([0]);
  return Gen.list(n, n, Gen.u8()).map((raw) {
    final parents = List<int>.filled(n, 0);
    for (var i = 1; i < n; i++) {
      parents[i] = raw[i] % i;
    }
    return parents;
  });
}

Gen<Tree> _tree(int minSize, int maxSize) =>
    Gen.integer(BigInt.from(minSize), BigInt.from(maxSize))
        .flatMap((n) => _parents(n.toInt()).map(_buildTree));

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('structural invariants hold', () {
      final verdict = checkProperty<Tree>(
        BigInt.from(101),
        PbtOptions(numTests: 200, seed: BigInt.from(101)),
        (rng) => _tree(1, 12).sample(rng),
        (t) {
          if (t.graph.edges().length != t.size() - 1) return false;
          if (t.graph.hasCycle()) return false;
          if (t.parent(t.root) != null) return false;
          for (final node in t.nodes()) {
            if (node == t.root) continue;
            if (t.parent(node) == null) return false;
            if (!t.hasNode(t.parent(node)!)) return false;
          }
          for (final leaf in t.leaves()) {
            if (!t.isLeaf(leaf)) return false;
          }
          return true;
        },
      );
      _check('invariants', verdict);
    });

    test('traversals permute the nodes with root anchors', () {
      final verdict = checkProperty<Tree>(
        BigInt.from(102),
        PbtOptions(numTests: 200, seed: BigInt.from(102)),
        (rng) => _tree(1, 12).sample(rng),
        (t) {
          final nodes = t.nodes();
          final pre = t.preorder();
          final post = t.postorder();
          final level = t.levelOrder();
          if (pre.length != nodes.length ||
              post.length != nodes.length ||
              level.length != nodes.length) {
            return false;
          }
          final sorted = List.of(nodes)..sort();
          for (final order in [pre, post, level]) {
            final copy = List.of(order)..sort();
            if (copy.join('\x00') != sorted.join('\x00')) return false;
          }
          return pre.first == t.root &&
              post.last == t.root &&
              level.first == t.root;
        },
      );
      _check('traversal-laws', verdict);
    });

    test('paths, depths, and lca agree', () {
      final gen = Gen.tuple2(_tree(1, 10), Gen.u8());
      final verdict = checkProperty<(Tree, int)>(
        BigInt.from(103),
        PbtOptions(numTests: 200, seed: BigInt.from(103)),
        (rng) => gen.sample(rng),
        (input) {
          final t = input.$1;
          final nodes = t.nodes();
          for (final node in nodes) {
            final path = t.pathTo(node);
            if (path.length != t.depth(node) + 1) return false;
            if (path.first != t.root || path.last != node) return false;
          }
          if (nodes.isEmpty) return false;
          final a = nodes[input.$2 % nodes.length];
          final b = nodes[(input.$2 ~/ 13) % nodes.length];
          if (t.lca(a, b) != t.lca(b, a)) return false;
          final ancestor = t.lca(a, b);
          if (!t.pathTo(a).contains(ancestor)) return false;
          if (!t.pathTo(b).contains(ancestor)) return false;
          if (t.depth(ancestor) > t.depth(a)) return false;
          if (t.depth(ancestor) > t.depth(b)) return false;
          return true;
        },
      );
      _check('path-lca', verdict);
    });

    test('subtree extracts consistently without modifying', () {
      final gen = Gen.tuple2(_tree(1, 10), Gen.u8());
      final verdict = checkProperty<(Tree, int)>(
        BigInt.from(104),
        PbtOptions(numTests: 200, seed: BigInt.from(104)),
        (rng) => gen.sample(rng),
        (input) {
          final t = input.$1;
          final nodes = t.nodes();
          final target = nodes[input.$2 % nodes.length];
          final before = t.size();
          final sub = t.subtree(target);
          if (t.size() != before) return false;
          if (sub.root != target) return false;
          // Every subtree node is a tree node with the same children.
          for (final node in sub.nodes()) {
            if (!t.hasNode(node)) return false;
            if (sub.children(node).join('\x00') !=
                t.children(node).join('\x00')) {
              return false;
            }
          }
          return true;
        },
      );
      _check('subtree-consistent', verdict);
    });

    test('removeSubtree restores invariants on the remainder', () {
      final gen = Gen.tuple2(_tree(2, 10), Gen.u8());
      final verdict = checkProperty<(Tree, int)>(
        BigInt.from(105),
        PbtOptions(numTests: 200, seed: BigInt.from(105)),
        (rng) => gen.sample(rng),
        (input) {
          final t = input.$1;
          final candidates = t.nodes().where((n) => n != t.root).toList();
          final target = candidates[input.$2 % candidates.length];
          final removed = t.subtree(target).size();
          final before = t.size();
          t.removeSubtree(target);
          if (t.size() != before - removed) return false;
          if (t.hasNode(target)) return false;
          if (t.graph.edges().length != t.size() - 1) return false;
          if (t.graph.hasCycle()) return false;
          for (final node in t.nodes()) {
            if (t.pathTo(node).length != t.depth(node) + 1) return false;
          }
          return true;
        },
      );
      _check('remove-restores', verdict);
    });

    test('mutation errors carry exact values', () {
      final gen = Gen.tuple2(_tree(1, 8), Gen.u8());
      final verdict = checkProperty<(Tree, int)>(
        BigInt.from(106),
        PbtOptions(numTests: 200, seed: BigInt.from(106)),
        (rng) => gen.sample(rng),
        (input) {
          final t = input.$1;
          final nodes = t.nodes();
          final existing = nodes[input.$2 % nodes.length];
          try {
            t.addChild('ghost-parent', 'fresh-child');
            return false;
          } on TreeError catch (e) {
            if (e != const NodeNotFound('ghost-parent')) return false;
          }
          try {
            t.addChild(t.root, existing);
            return false;
          } on TreeError catch (e) {
            if (e != DuplicateNode(existing)) return false;
          }
          try {
            t.removeSubtree(t.root);
            return false;
          } on TreeError catch (e) {
            if (e != const RootRemoval()) return false;
          }
          return true;
        },
      );
      _check('error-values', verdict);
    });
  });
}
