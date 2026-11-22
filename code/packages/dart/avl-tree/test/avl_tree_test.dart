// Tests for coding_adventures_avl_tree (DT08).
//
// Part 1 mirrors the reference suites: all four rotation shapes with exact
// roots, search/order statistics, deletions, predecessor/successor, and
// validity metadata. Part 2 is a seeded model-based sweep: 1,500 mixed
// inserts/deletes against a sorted-list oracle with validity, order, size,
// rank, and kth agreement checked after every step. Part 3 covers
// immutability (old versions never change), duplicates, string keys, and
// free-function entry points.

import 'dart:math';

import 'package:coding_adventures_avl_tree/coding_adventures_avl_tree.dart';
import 'package:test/test.dart';

void main() {
  group('reference behavior', () {
    test('empty tree behaves', () {
      final tree = AvlTree<int>.empty();
      expect(tree.size(), 0);
      expect(tree.height(), -1);
      expect(tree.search(1), isNull);
      expect(tree.contains(1), isFalse);
      expect(tree.minValue(), isNull);
      expect(tree.maxValue(), isNull);
      expect(tree.predecessor(1), isNull);
      expect(tree.successor(1), isNull);
      expect(tree.kthSmallest(1), isNull);
      expect(tree.rank(1), 0);
      expect(tree.toSortedArray(), isEmpty);
      expect(tree.isValidBst(), isTrue);
      expect(tree.isValidAvl(), isTrue);
      expect(tree.root(), isNull);
    });

    test('LL rotation rebalances (30, 20, 10)', () {
      final tree = AvlTree<int>.empty().insert(30).insert(20).insert(10);
      expect(tree.root()?.value, 20);
      expect(tree.isValidAvl(), isTrue);
      expect(tree.toSortedArray(), [10, 20, 30]);
    });

    test('RR rotation rebalances (10, 20, 30)', () {
      final tree = AvlTree<int>.empty().insert(10).insert(20).insert(30);
      expect(tree.root()?.value, 20);
      expect(tree.isValidAvl(), isTrue);
      expect(tree.toSortedArray(), [10, 20, 30]);
    });

    test('LR rotation rebalances (30, 10, 20)', () {
      final tree = AvlTree<int>.empty().insert(30).insert(10).insert(20);
      expect(tree.root()?.value, 20);
      expect(tree.isValidAvl(), isTrue);
    });

    test('RL rotation rebalances (10, 30, 20)', () {
      final tree = AvlTree<int>.empty().insert(10).insert(30).insert(20);
      expect(tree.root()?.value, 20);
      expect(tree.isValidAvl(), isTrue);
    });

    test('search and order statistics work', () {
      final tree = AvlTree<int>.empty()
          .insert(8)
          .insert(3)
          .insert(10)
          .insert(1)
          .insert(6)
          .insert(14)
          .insert(4)
          .insert(7);
      expect(tree.contains(6), isTrue);
      expect(tree.contains(99), isFalse);
      expect(tree.search(6)?.value, 6);
      expect(tree.minValue(), 1);
      expect(tree.maxValue(), 14);
      expect(tree.rank(6), 3);
      expect(tree.kthSmallest(4), 6);
      expect(tree.kthSmallest(0), isNull);
      expect(tree.kthSmallest(99), isNull);
      expect(tree.toSortedArray(), [1, 3, 4, 6, 7, 8, 10, 14]);
      expect(tree.isValidBst(), isTrue);
      expect(tree.isValidAvl(), isTrue);
      expect(tree.size(), 8);
      expect(tree.balanceFactor(tree.root()!), inInclusiveRange(-1, 1));
    });

    test('predecessor and successor walk neighbors', () {
      final tree =
          AvlTree<int>.empty().insert(10).insert(20).insert(30).insert(40);
      expect(tree.predecessor(30), 20);
      expect(tree.successor(30), 40);
      expect(tree.predecessor(10), isNull);
      expect(tree.successor(40), isNull);
      // Absent values resolve against their would-be position.
      expect(tree.predecessor(25), 20);
      expect(tree.successor(25), 30);
    });

    test('deletions cover leaf, single-child, and two-child cases', () {
      var tree = AvlTree<int>.empty();
      for (final v in [50, 30, 70, 20, 40, 60, 80]) {
        tree = tree.insert(v);
      }
      // Leaf.
      tree = tree.delete(20);
      expect(tree.contains(20), isFalse);
      expect(tree.isValidAvl(), isTrue);
      // One child (30 now has only right child 40).
      tree = tree.delete(30);
      expect(tree.contains(30), isFalse);
      expect(tree.isValidAvl(), isTrue);
      // Two children (root 50).
      tree = tree.delete(50);
      expect(tree.contains(50), isFalse);
      expect(tree.isValidAvl(), isTrue);
      expect(tree.toSortedArray(), [40, 60, 70, 80]);
      // Missing key is a no-op.
      final before = tree.toSortedArray();
      tree = tree.delete(999);
      expect(tree.toSortedArray(), before);
      expect(tree.isValidAvl(), isTrue);
    });

    test('rank counts strictly smaller values', () {
      final tree = AvlTree<int>.empty().insert(10).insert(20).insert(30);
      expect(tree.rank(5), 0);
      expect(tree.rank(10), 0);
      expect(tree.rank(15), 1);
      expect(tree.rank(30), 2);
      expect(tree.rank(99), 3);
    });
  });

  group('model-based sweep', () {
    test('1,500 mixed ops agree with a sorted-list oracle', () {
      final rng = Random(0xA71);
      var tree = AvlTree<int>.empty();
      final oracle = <int>[];
      for (var step = 0; step < 1500; step++) {
        final value = rng.nextInt(200);
        if (rng.nextBool()) {
          tree = tree.insert(value);
          if (!oracle.contains(value)) {
            oracle.add(value);
            oracle.sort();
          }
        } else {
          final wasPresent = oracle.contains(value);
          tree = tree.delete(value);
          expect(tree.contains(value), isFalse);
          oracle.remove(value);
          if (wasPresent) {
            expect(tree.size(), oracle.length);
          }
        }
        expect(tree.toSortedArray(), oracle, reason: 'step $step order');
        expect(tree.size(), oracle.length, reason: 'step $step size');
        expect(tree.isValidAvl(), isTrue, reason: 'step $step avl');
        expect(tree.height() < 20, isTrue,
            reason: 'step $step logarithmic height');
        if (oracle.isNotEmpty) {
          final sample = oracle[rng.nextInt(oracle.length)];
          expect(tree.rank(sample), oracle.indexOf(sample));
          final k = 1 + rng.nextInt(oracle.length);
          expect(tree.kthSmallest(k), oracle[k - 1]);
          expect(tree.minValue(), oracle.first);
          expect(tree.maxValue(), oracle.last);
        }
      }
    });
  });

  group('edge cases', () {
    test('old versions never change', () {
      final v0 = AvlTree<int>.empty();
      final v1 = v0.insert(1);
      final v2 = v1.insert(2);
      final v3 = v2.delete(1);
      expect(v0.toSortedArray(), isEmpty);
      expect(v1.toSortedArray(), [1]);
      expect(v2.toSortedArray(), [1, 2]);
      expect(v3.toSortedArray(), [2]);
      for (final v in [v0, v1, v2, v3]) {
        expect(v.isValidAvl(), isTrue);
      }
    });

    test('duplicate inserts are no-ops', () {
      final tree = AvlTree<int>.empty().insert(5).insert(5).insert(5);
      expect(tree.toSortedArray(), [5]);
      expect(tree.size(), 1);
      expect(tree.isValidAvl(), isTrue);
    });

    test('string keys sort lexicographically', () {
      final tree =
          AvlTree<String>.empty().insert('pear').insert('apple').insert('fig');
      expect(tree.toSortedArray(), ['apple', 'fig', 'pear']);
      expect(tree.rank('fig'), 1);
      expect(tree.kthSmallest(3), 'pear');
      expect(tree.predecessor('fig'), 'apple');
      expect(tree.successor('fig'), 'pear');
    });

    test('free functions operate on nodes', () {
      final root = avlInsert(avlInsert(avlInsert(null, 2), 1), 3);
      expect(avlSearch(root, 2)?.value, 2);
      expect(avlSearch(root, 9), isNull);
      expect(avlHeight(root), 1);
      expect(avlSize(root), 3);
      expect(balanceFactorOf(root!), 0);
      expect(isValidAvlRoot(root), isTrue);
      expect(avlHeight(null), -1);
      expect(avlSize(null), 0);
      expect(isValidAvlRoot(null), isTrue);
      final smaller = avlDelete(root, 2)!;
      expect(avlSize(smaller), 2);
      // Rotations without the needed child are no-ops.
      final leaf = AvlNode(1);
      expect(identical(rotateLeft(leaf), leaf), isTrue);
      expect(identical(rotateRight(leaf), leaf), isTrue);
    });

    test('rotations rebalance directly', () {
      // Right-heavy chain 1-2-3: left rotation lifts 2.
      final chain = AvlNode(1)..right = (AvlNode(2)..right = AvlNode(3));
      final balanced = rotateLeft(chain);
      expect(balanced.value, 2);
      // Left-heavy chain 3-2-1: right rotation lifts 2.
      final chain2 = AvlNode(3)..left = (AvlNode(2)..left = AvlNode(1));
      expect(rotateRight(chain2).value, 2);
      // rebalance passes through balanced nodes.
      final even = AvlNode(2)
        ..left = AvlNode(1)
        ..right = AvlNode(3);
      expect(identical(rebalance(even), even), isTrue);
    });
  });
}

Matcher inInclusiveRange(int lo, int hi) =>
    predicate<int>((v) => v >= lo && v <= hi, 'in [$lo, $hi]');
