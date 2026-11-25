/// Script-driven differential fuzz campaign for `coding_adventures_tree`,
/// using the in-lane `fuzz` package: hostile byte strings decode into
/// mutation scripts (adds, prunes) interleaved with full-state queries, and
/// an independent parent-map model predicts every observable — node set,
/// parents, children, depths, paths, traversals, LCA, subtree shapes.
/// Only the exact expected [TreeError] may ever escape. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_tree/coding_adventures_tree.dart';
import 'package:test/test.dart';

/// Independent model: a parent map (root maps to null).
class _Model {
  final String root;
  final Map<String, String?> parentOf = {};

  _Model(this.root) {
    parentOf[root] = null;
  }

  Set<String> get nodes => parentOf.keys.toSet();

  List<String> childrenOf(String node) {
    final result = parentOf.entries
        .where((e) => e.value == node)
        .map((e) => e.key)
        .toList();
    result.sort();
    return result;
  }

  int depthOf(String node) {
    var d = 0;
    var current = node;
    while (current != root) {
      current = parentOf[current]!;
      d++;
    }
    return d;
  }

  List<String> pathTo(String node) {
    final path = <String>[];
    var current = node;
    while (true) {
      path.add(current);
      if (current == root) break;
      current = parentOf[current]!;
    }
    return path.reversed.toList();
  }

  List<String> _preorder(String node) {
    final result = [node];
    for (final child in childrenOf(node)) {
      result.addAll(_preorder(child));
    }
    return result;
  }

  List<String> _postorder(String node) {
    final result = <String>[];
    for (final child in childrenOf(node)) {
      result.addAll(_postorder(child));
    }
    result.add(node);
    return result;
  }

  List<String> _levelOrder() {
    final result = <String>[];
    final queue = [root];
    while (queue.isNotEmpty) {
      final node = queue.removeAt(0);
      result.add(node);
      queue.addAll(childrenOf(node));
    }
    return result;
  }

  String lca(String a, String b) {
    final pathA = pathTo(a);
    final pathB = pathTo(b);
    var ancestor = root;
    final shared = pathA.length < pathB.length ? pathA.length : pathB.length;
    for (var i = 0; i < shared; i++) {
      if (pathA[i] == pathB[i]) {
        ancestor = pathA[i];
      } else {
        break;
      }
    }
    return ancestor;
  }

  /// Deletes [node] and its descendants from the model.
  void prune(String node) {
    final doomed = <String>{node};
    var grew = true;
    while (grew) {
      grew = false;
      for (final entry in parentOf.entries) {
        if (entry.value != null &&
            doomed.contains(entry.value) &&
            !doomed.contains(entry.key)) {
          doomed.add(entry.key);
          grew = true;
        }
      }
    }
    for (final name in doomed) {
      parentOf.remove(name);
    }
  }
}

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw.
class _Decoder {
  final Uint8List input;
  int cursor = 0;

  _Decoder(this.input);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  /// A node name: pooled names (forcing collisions) or fresh/odd ones.
  String takeName(int freshCounter) {
    switch (takeByte() % 5) {
      case 0:
      case 1:
      case 2:
        return 'n${takeByte() % 12}';
      case 3:
        return 'x$freshCounter';
      default:
        return ['', ' ', 'café', 'a/b'][takeByte() % 4];
    }
  }
}

bool _listsEqual(List<String> a, List<String> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final tree = Tree('n0');
    final model = _Model('n0');
    var freshCounter = 0;

    bool checkAll() {
      final wantNodes = model.nodes.toList()..sort();
      if (!_listsEqual(tree.nodes(), wantNodes)) return false;
      if (tree.size() != wantNodes.length) return false;
      if (tree.graph.edges().length != wantNodes.length - 1) return false;
      if (tree.graph.hasCycle()) return false;
      for (final node in wantNodes) {
        if (tree.parent(node) != model.parentOf[node]) return false;
        if (!_listsEqual(tree.children(node), model.childrenOf(node))) {
          return false;
        }
        if (tree.depth(node) != model.depthOf(node)) return false;
        if (!_listsEqual(tree.pathTo(node), model.pathTo(node))) {
          return false;
        }
        if (tree.isLeaf(node) != model.childrenOf(node).isEmpty) {
          return false;
        }
      }
      if (!_listsEqual(tree.preorder(), model._preorder(model.root))) {
        return false;
      }
      if (!_listsEqual(tree.postorder(), model._postorder(model.root))) {
        return false;
      }
      if (!_listsEqual(tree.levelOrder(), model._levelOrder())) {
        return false;
      }
      // LCA over a sample of pairs (full quadratic sweep each op would
      // dominate the campaign budget on big trees).
      for (var i = 0; i < wantNodes.length; i += 3) {
        for (var j = 0; j < wantNodes.length; j += 5) {
          if (tree.lca(wantNodes[i], wantNodes[j]) !=
              model.lca(wantNodes[i], wantNodes[j])) {
            return false;
          }
        }
      }
      return true;
    }

    final ops = d.takeByte() % 20 + 1;
    for (var i = 0; i < ops; i++) {
      final op = d.takeByte() % 6;
      cov.recordEdge(op);
      switch (op) {
        case 0 || 1:
          final parent = d.takeName(freshCounter);
          final child = d.takeName(freshCounter);
          freshCounter++;
          final parentMissing = !model.nodes.contains(parent);
          final childPresent = model.nodes.contains(child);
          try {
            tree.addChild(parent, child);
            if (parentMissing || childPresent) {
              return const OutcomeCrash(
                  'error-missing', ['addChild-should-throw']);
            }
            model.parentOf[child] = parent;
          } on TreeError catch (e) {
            final want = parentMissing
                ? NodeNotFound(parent)
                : childPresent
                    ? DuplicateNode(child)
                    : null;
            if (want == null || e != want) {
              return const OutcomeCrash(
                  'error-divergence', ['addChild-wrong-error']);
            }
          }
        case 2:
          final target = d.takeName(freshCounter);
          freshCounter++;
          final missing = !model.nodes.contains(target);
          final isRoot = target == model.root;
          try {
            tree.removeSubtree(target);
            if (missing || isRoot) {
              return const OutcomeCrash(
                  'error-missing', ['remove-should-throw']);
            }
            model.prune(target);
          } on TreeError catch (e) {
            final want = missing
                ? NodeNotFound(target)
                : isRoot
                    ? const RootRemoval()
                    : null;
            if (want == null || e != want) {
              return const OutcomeCrash(
                  'error-divergence', ['remove-wrong-error']);
            }
          }
        case 3:
          final target = d.takeName(freshCounter);
          freshCounter++;
          final missing = !model.nodes.contains(target);
          try {
            final sub = tree.subtree(target);
            if (missing) {
              return const OutcomeCrash(
                  'error-missing', ['subtree-should-throw']);
            }
            // Subtree shape: same root, same nodes, same children.
            final wantNodes = <String>{};
            final queue = [target];
            while (queue.isNotEmpty) {
              final node = queue.removeAt(0);
              wantNodes.add(node);
              queue.addAll(model.childrenOf(node));
            }
            final gotNodes = sub.nodes().toSet();
            if (gotNodes.length != wantNodes.length ||
                !gotNodes.containsAll(wantNodes)) {
              return const OutcomeCrash('model-divergence', ['subtree-nodes']);
            }
            if (sub.root != target) {
              return const OutcomeCrash('model-divergence', ['subtree-root']);
            }
            for (final node in gotNodes) {
              if (!_listsEqual(sub.children(node), model.childrenOf(node))) {
                return const OutcomeCrash(
                    'model-divergence', ['subtree-children']);
              }
            }
          } on TreeError catch (e) {
            if (!missing || e != NodeNotFound(target)) {
              return const OutcomeCrash(
                  'error-divergence', ['subtree-wrong-error']);
            }
          }
        default:
          // Pure query sweep: no mutation, full comparison.
          break;
      }
      if (!checkAll()) {
        return const OutcomeCrash('model-divergence', ['state-mismatch']);
      }
    }
    cov.recordEdge(10 + (model.nodes.length % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([8, 0, 1, 2, 0, 0, 1, 1, 3]),
      Uint8List.fromList([15, 0, 5, 5, 0, 9, 9, 1, 2, 3, 2, 4, 4, 1, 0, 0]),
      Uint8List.fromList([20, 2, 7, 7, 0, 1, 1, 3, 3, 3, 5, 5, 5]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 107);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(107)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, errors exact, no crashes', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(5));
    });
  });
}
