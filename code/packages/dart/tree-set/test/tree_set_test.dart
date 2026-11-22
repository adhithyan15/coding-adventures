// Tests for coding_adventures_tree_set.
//
// Part 1 mirrors the reference suites: ordered operations, ranges, and the
// full set algebra with exact vectors. Part 2 is a seeded model-based sweep:
// random sets checked against Dart's Set oracle for every algebra and
// predicate after each step. Part 3 covers backends, aliases, empties, and
// immutability.

import 'dart:math';

import 'package:coding_adventures_tree_set/coding_adventures_tree_set.dart';
import 'package:test/test.dart';

void main() {
  group('reference behavior', () {
    test('ordered set operations work', () {
      final set = TreeSet<int>.fromList([7, 3, 9, 1, 5, 3]);
      expect(set.toSortedArray(), [1, 3, 5, 7, 9]);
      expect(set.toList(), [1, 3, 5, 7, 9]);
      expect(set.keys.toList(), [1, 3, 5, 7, 9]);
      expect(set.length, 5);
      expect(set.size(), 5);
      expect(set.isEmpty, isFalse);
      expect(set.isNotEmpty, isTrue);
      expect(set.minValue(), 1);
      expect(set.maxValue(), 9);
      expect(set.first(), 1);
      expect(set.last(), 9);
      expect(set.rank(7), 3);
      expect(set.kthSmallest(3), 5);
      expect(set.kthSmallest(0), isNull);
      expect(set.predecessor(5), 3);
      expect(set.successor(5), 7);
      expect(set.range(3, 7, true), [3, 5, 7]);
      expect(set.range(3, 7, false), [5]);
      expect(set.contains(5), isTrue);
      expect(set.contains(6), isFalse);

      final removed = set.remove(5);
      expect(removed.toSortedArray(), [1, 3, 7, 9]);
      expect(set.toSortedArray(), [1, 3, 5, 7, 9]);
      expect(set.delete(5).toSortedArray(), [1, 3, 7, 9]);
    });

    test('set algebra matches exact vectors', () {
      final left = TreeSet<int>.fromList([1, 2, 3, 5]);
      final right = TreeSet<int>.fromList([3, 4, 5, 6]);
      expect(left.union(right).toSortedArray(), [1, 2, 3, 4, 5, 6]);
      expect(left.intersection(right).toSortedArray(), [3, 5]);
      expect(left.difference(right).toSortedArray(), [1, 2]);
      expect(left.symmetricDifference(right).toSortedArray(), [1, 2, 4, 6]);
      expect(left.isSubset(left.union(right)), isTrue);
      expect(
          left.isSuperset(
              left.intersection(right).union(TreeSet<int>.fromList([1, 2]))),
          isTrue);
      expect(left.isDisjoint(TreeSet<int>.fromList([8, 9])), isTrue);
      expect(left.isDisjoint(right), isFalse);
      expect(left.equals(TreeSet<int>.fromList([1, 2, 3, 5])), isTrue);
      expect(left.equals(right), isFalse);
      expect(left.equals(TreeSet<int>.fromList([1, 2, 3])), isFalse);
    });

    test('empty sets behave', () {
      final empty = TreeSet<int>.empty();
      expect(TreeSet<int>().toSortedArray(), isEmpty);
      expect(TreeSet<int>.fromSortedArray([]).toSortedArray(), isEmpty);
      expect(empty.length, 0);
      expect(empty.isEmpty, isTrue);
      expect(empty.minValue(), isNull);
      expect(empty.maxValue(), isNull);
      expect(empty.first(), isNull);
      expect(empty.last(), isNull);
      expect(empty.predecessor(1), isNull);
      expect(empty.successor(1), isNull);
      expect(empty.kthSmallest(1), isNull);
      expect(empty.rank(1), 0);
      expect(empty.range(1, 9, true), isEmpty);
      expect(empty.toString(), 'TreeSet([])');
      final other = TreeSet<int>.fromList([1]);
      expect(empty.union(other).toSortedArray(), [1]);
      expect(empty.intersection(other).toSortedArray(), isEmpty);
      expect(empty.difference(other).toSortedArray(), isEmpty);
      expect(other.difference(empty).toSortedArray(), [1]);
      expect(empty.symmetricDifference(other).toSortedArray(), [1]);
      expect(empty.isSubset(other), isTrue);
      expect(other.isSubset(empty), isFalse);
      expect(empty.isSuperset(other), isFalse);
      expect(other.isSuperset(empty), isTrue);
      expect(empty.isDisjoint(other), isTrue);
      expect(empty.equals(TreeSet<int>.empty()), isTrue);
      expect(empty.remove(1).toSortedArray(), isEmpty);
    });

    test('ranges honor inclusivity and order', () {
      final set = TreeSet<int>.fromList([5, 12, 20, 37, 42]);
      expect(set.range(20, 42, true), [20, 37, 42]);
      expect(set.range(20, 42, false), [37]);
      expect(set.range(42, 20, true), isEmpty);
      expect(set.range(20, 20, true), [20]);
      expect(set.range(20, 20, false), isEmpty);
      expect(set.range(6, 11, true), isEmpty);
    });
  });

  group('model-based sweep', () {
    test('algebra agrees with Set oracle over 500 random pairs', () {
      final rng = Random(0x7E7);
      for (var step = 0; step < 500; step++) {
        final aVals = {
          for (var i = 0, n = rng.nextInt(12); i < n; i++) rng.nextInt(30)
        };
        final bVals = {
          for (var i = 0, n = rng.nextInt(12); i < n; i++) rng.nextInt(30)
        };
        final a = TreeSet<int>.fromList(aVals);
        final b = TreeSet<int>.fromList(bVals);
        List<int> sorted(Set<int> s) => s.toList()..sort();
        expect(a.union(b).toSortedArray(), sorted(aVals.union(bVals)),
            reason: 'step $step union');
        expect(a.intersection(b).toSortedArray(),
            sorted(aVals.intersection(bVals)),
            reason: 'step $step intersection');
        expect(a.difference(b).toSortedArray(), sorted(aVals.difference(bVals)),
            reason: 'step $step difference');
        expect(a.symmetricDifference(b).toSortedArray(),
            sorted(aVals.difference(bVals).union(bVals.difference(aVals))),
            reason: 'step $step symdiff');
        expect(a.isSubset(b), bVals.containsAll(aVals),
            reason: 'step $step subset');
        expect(a.isSuperset(b), aVals.containsAll(bVals),
            reason: 'step $step superset');
        expect(a.isDisjoint(b), aVals.intersection(bVals).isEmpty,
            reason: 'step $step disjoint');
        expect(a.equals(b),
            aVals.length == bVals.length && aVals.containsAll(bVals),
            reason: 'step $step equals');
        // Ranges against a sorted-list oracle.
        final lo = rng.nextInt(30);
        final hi = rng.nextInt(30);
        final inclusive = rng.nextBool();
        final want = sorted(aVals)
            .where((v) => inclusive ? (v >= lo && v <= hi) : (v > lo && v < hi))
            .toList();
        expect(a.range(lo, hi, inclusive), want, reason: 'step $step range');
      }
    });
  });

  group('edge cases', () {
    test('backend swaps and delegates', () {
      final set = TreeSet<int>.withBackend(AvlTreeBackend<int>());
      expect(set.toSortedArray(), isEmpty);
      final grown = set.insert(4).insert(2);
      expect(grown.toSortedArray(), [2, 4]);
      expect(grown.backend.size(), 2);
      expect(grown.backend.rank(4), 1);
      expect(grown.backend.toSortedArray(), [2, 4]);
      expect(grown.backend.minValue(), 2);
      expect(grown.backend.maxValue(), 4);
      expect(grown.backend.predecessor(4), 2);
      expect(grown.backend.successor(2), 4);
      expect(grown.backend.kthSmallest(2), 4);
      expect(grown.backend.contains(2), isTrue);
      expect(grown.backend.delete(2).toSortedArray(), [4]);
    });

    test('old versions never change', () {
      final v0 = TreeSet<int>.empty();
      final v1 = v0.insert(1);
      final v2 = v1.insert(2);
      final v3 = v2.remove(1);
      expect(v0.toSortedArray(), isEmpty);
      expect(v1.toSortedArray(), [1]);
      expect(v2.toSortedArray(), [1, 2]);
      expect(v3.toSortedArray(), [2]);
    });

    test('duplicate inserts are no-ops', () {
      final set = TreeSet<int>.empty().insert(5).insert(5).insert(5);
      expect(set.toSortedArray(), [5]);
      expect(set.length, 1);
    });

    test('string keys sort lexicographically', () {
      final set = TreeSet<String>.fromList(['pear', 'apple', 'fig']);
      expect(set.toList(), ['apple', 'fig', 'pear']);
      expect(set.rank('fig'), 1);
      expect(set.kthSmallest(3), 'pear');
      expect(set.range('b', 'g', true), ['fig']);
      expect(set.union(TreeSet<String>.fromList(['kiwi'])).toList(),
          ['apple', 'fig', 'kiwi', 'pear']);
    });
  });
}
