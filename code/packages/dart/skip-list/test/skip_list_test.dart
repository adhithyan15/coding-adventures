// Tests for coding_adventures_skip_list (DT20).
//
// Part 1 mirrors the reference suites: empty behavior, insert/search/delete,
// sorted order, rank/byRank, inclusive/exclusive ranges, params, and tuning
// accessors. Part 2 is a seeded model-based sweep: 2,000 mixed operations
// against a plain-map oracle with sorted-order invariants checked after
// every step. Part 3 covers nullable values, string keys, and snapshot
// isolation (mutating the list never affects returned snapshots).

import 'dart:math';

import 'package:coding_adventures_skip_list/coding_adventures_skip_list.dart';
import 'package:test/test.dart';

void main() {
  group('reference behavior', () {
    test('empty list behaves', () {
      final sl = SkipList<int, int>();
      expect(sl.length, 0);
      expect(sl.size(), 0);
      expect(sl.isEmpty, isTrue);
      expect(sl.isNotEmpty, isFalse);
      expect(sl.search(42), isNull);
      expect(sl.contains(42), isFalse);
      expect(sl.containsKey(42), isFalse);
      expect(sl.rank(42), isNull);
      expect(sl.byRank(0), isNull);
      expect(sl.rangeQuery(1, 10, true), isEmpty);
      expect(sl.range(1, 10, false), isEmpty);
      expect(sl.toList(), isEmpty);
      expect(sl.entries(), isEmpty);
      expect(sl.entriesList(), isEmpty);
      expect(sl.min(), isNull);
      expect(sl.max(), isNull);
      expect(sl.delete(42), isFalse);
      expect(sl.toString(), 'SkipList([])');
      expect(sl.currentMax(), 1);
    });

    test('insert, search, delete, and reinsert update', () {
      final sl = SkipList<int, String>();
      sl.insert(42, 'answer');
      sl.insert(7, 'lucky');
      expect(sl.search(42), 'answer');
      expect(sl.search(7), 'lucky');
      expect(sl.contains(42), isTrue);
      expect(sl.delete(42), isTrue);
      expect(sl.contains(42), isFalse);
      expect(sl.search(42), isNull);
      expect(sl.delete(42), isFalse);
      sl.insert(7, 'changed');
      expect(sl.search(7), 'changed');
      expect(sl.length, 1);
    });

    test('iteration and toList are sorted', () {
      final sl = SkipList<int, int>();
      for (final key in [3, 1, 2]) {
        sl.insert(key, key * 10);
      }
      expect(sl.toList(), [1, 2, 3]);
      expect(sl.keys.toList(), [1, 2, 3]);
      expect(sl.min(), 1);
      expect(sl.max(), 3);
    });

    test('rank and byRank use 0-based sorted order', () {
      final sl = SkipList<int, int>();
      for (final key in [10, 20, 30, 40, 50]) {
        sl.insert(key, key * 2);
      }
      expect(sl.rank(10), 0);
      expect(sl.rank(30), 2);
      expect(sl.rank(99), isNull);
      expect(sl.byRank(0), 10);
      expect(sl.byRank(3), 40);
      expect(sl.byRank(10), isNull);
      expect(sl.byRank(-1), isNull);
    });

    test('range queries honor inclusivity', () {
      final sl = SkipList<int, int>();
      for (final key in [5, 12, 20, 37, 42, 50, 55, 63]) {
        sl.insert(key, key * 10);
      }
      expect(
        sl.rangeQuery(20, 55, true).map((e) => e.$1).toList(),
        [20, 37, 42, 50, 55],
      );
      expect(
        sl.rangeQuery(20, 55, false).map((e) => e.$1).toList(),
        [37, 42, 50],
      );
      expect(
        sl.range(20, 55, true).map((e) => e.$2).toList(),
        [200, 370, 420, 500, 550],
      );
      expect(sl.rangeQuery(55, 20, true), isEmpty,
          reason: 'lo > hi yields nothing');
      expect(sl.rangeQuery(37, 37, true).map((e) => e.$1).toList(), [37]);
      expect(sl.rangeQuery(37, 37, false), isEmpty);
    });

    test('entries carry pairs in sorted order', () {
      final sl = SkipList<int, String>();
      sl.insert(2, 'b');
      sl.insert(1, 'a');
      expect(sl.entries(), [(1, 'a'), (2, 'b')]);
      expect(sl.entriesList(), [(1, 'a'), (2, 'b')]);
    });

    test('parameters default, store, and sanitize', () {
      final d = SkipList<int, int>();
      expect(d.maxLevel(), 32);
      expect(d.probability(), 0.5);
      final custom = SkipList<int, int>.withParams(8, 0.9);
      expect(custom.maxLevel(), 8);
      expect(custom.probability(), 0.9);
      expect(custom.currentMax(), 1);
      final alias = SkipList<int, int>.newWithParams(8, 0.9);
      expect(alias.maxLevel(), 8);
      final bad = SkipList<int, int>.withParams(0, 7.0);
      expect(bad.maxLevel(), 1);
      expect(bad.probability(), 0.5);
      final nan = SkipList<int, int>.withParams(4, double.nan);
      expect(nan.probability(), 0.5);
    });

    test('currentMax estimates height from size', () {
      final sl = SkipList<int, int>();
      expect(sl.currentMax(), 1);
      for (var i = 0; i < 100; i++) {
        sl.insert(i, i);
      }
      // ceil(log(100)/log(2)) = 7 with p = 0.5.
      expect(sl.currentMax(), 7);
      final capped = SkipList<int, int>.withParams(4, 0.5);
      for (var i = 0; i < 100; i++) {
        capped.insert(i, i);
      }
      expect(capped.currentMax(), 4);
    });
  });

  group('model-based sweep', () {
    test('2,000 mixed ops agree with a map oracle', () {
      // Deterministic seed: same operations every run. After each step the
      // sorted order, size, and a sampled rank/range agree with the oracle.
      final rng = Random(0x51A9);
      final sl = SkipList<int, int>();
      final oracle = <int, int>{};
      for (var step = 0; step < 2000; step++) {
        final op = rng.nextInt(100);
        final key = rng.nextInt(120);
        if (op < 45) {
          final value = rng.nextInt(1000);
          sl.insert(key, value);
          oracle[key] = value;
        } else if (op < 65) {
          expect(sl.delete(key), oracle.containsKey(key),
              reason: 'step $step delete $key');
          oracle.remove(key);
        } else if (op < 85) {
          expect(sl.search(key), oracle[key], reason: 'step $step search');
        } else {
          final lo = rng.nextInt(120);
          final hi = rng.nextInt(120);
          final inclusive = rng.nextBool();
          final want = oracle.keys
              .where(
                  (k) => inclusive ? (k >= lo && k <= hi) : (k > lo && k < hi))
              .toList()
            ..sort();
          expect(
              sl.rangeQuery(lo, hi, inclusive).map((e) => e.$1).toList(), want,
              reason: 'step $step range');
        }
        final sortedKeys = oracle.keys.toList()..sort();
        expect(sl.toList(), sortedKeys, reason: 'step $step order');
        expect(sl.length, oracle.length, reason: 'step $step size');
        expect(sl.isEmpty, oracle.isEmpty);
        if (sortedKeys.isNotEmpty) {
          final sample = sortedKeys[rng.nextInt(sortedKeys.length)];
          expect(sl.rank(sample), sortedKeys.indexOf(sample));
          expect(sl.min(), sortedKeys.first);
          expect(sl.max(), sortedKeys.last);
        }
      }
    });
  });

  group('edge cases', () {
    test('nullable values round-trip and delete cleanly', () {
      final sl = SkipList<int, int?>();
      sl.insert(1, null);
      expect(sl.contains(1), isTrue);
      expect(sl.search(1), isNull);
      expect(sl.delete(1), isTrue);
      expect(sl.contains(1), isFalse);
      expect(sl.delete(1), isFalse);
    });

    test('string keys sort lexicographically', () {
      final sl = SkipList<String, int>();
      sl.insert('pear', 1);
      sl.insert('apple', 2);
      sl.insert('fig', 3);
      expect(sl.toList(), ['apple', 'fig', 'pear']);
      expect(sl.rank('fig'), 1);
      expect(sl.byRank(2), 'pear');
      expect(sl.rangeQuery('b', 'g', true).map((e) => e.$1).toList(), ['fig']);
    });

    test('snapshots isolate later mutations', () {
      final sl = SkipList<int, int>();
      sl.insert(1, 10);
      sl.insert(2, 20);
      final keys = sl.toList();
      final pairs = sl.entries();
      sl.insert(3, 30);
      sl.delete(1);
      expect(keys, [1, 2]);
      expect(pairs, [(1, 10), (2, 20)]);
    });

    test('delete updates size and extrema', () {
      final sl = SkipList<int, int>();
      for (var i = 0; i < 10; i++) {
        sl.insert(i, i);
      }
      expect(sl.delete(0), isTrue);
      expect(sl.delete(9), isTrue);
      expect(sl.min(), 1);
      expect(sl.max(), 8);
      expect(sl.length, 8);
      expect(sl.size(), 8);
      expect(sl.isNotEmpty, isTrue);
    });
  });
}
