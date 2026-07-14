import 'dart:math';

import 'package:coding_adventures_b_plus_tree/coding_adventures_b_plus_tree.dart';
import 'package:test/test.dart';

final class _Key {
  const _Key(this.value);

  final int value;
}

void main() {
  group('construction and metadata', () {
    test('rejects a minimum degree below two', () {
      expect(() => BPlusTree<int, String>(1), throwsArgumentError);

      final tree = BPlusTree<int, String>();
      expect(tree.minimumDegree, 2);
      expect(tree.count, 0);
      expect(tree.size, 0);
      expect(tree.isEmpty, isTrue);
      expect(tree.height, 0);
      expect(tree.isValid(), isTrue);
      expect(tree.toString(), 'BPlusTree(t=2, size=0, height=0)');
    });
  });

  group('insert and lookup', () {
    test('upserts values without changing count', () {
      final tree = BPlusTree<int, String>(3);
      for (final key in [10, 5, 20, 1, 15, 30]) {
        tree.insert(key, 'v$key');
      }
      tree.insert(15, 'updated');

      expect(tree.count, 6);
      expect(tree.contains(20), isTrue);
      expect(tree.contains(99), isFalse);
      expect(tree.search(15), 'updated');
      expect(tree.search(99), isNull);
      expect(tree.isValid(), isTrue);
    });

    test('contains distinguishes a nullable value from a missing key', () {
      final tree = BPlusTree<int, String?>()..insert(42, null);

      expect(tree.contains(42), isTrue);
      expect(tree.search(42), isNull);
      expect(tree.contains(7), isFalse);
      expect(tree.isValid(), isTrue);
    });

    test('accepts a custom comparator for non-Comparable keys', () {
      final tree = BPlusTree<_Key, String>(
        2,
        (left, right) => left.value.compareTo(right.value),
      )
        ..insert(const _Key(2), 'two')
        ..insert(const _Key(1), 'one');

      expect(tree.search(const _Key(2)), 'two');
      expect(tree.fullScan().map((entry) => entry.$1.value), [1, 2]);
      expect(tree.isValid(), isTrue);
    });

    test('sequential inserts create linked leaves and sorted scans', () {
      final tree = BPlusTree<int, String>();
      for (var key = 1; key <= 3; key++) {
        tree.insert(key, 'v$key');
      }
      expect(tree.height, 0);

      for (var key = 4; key <= 50; key++) {
        tree.insert(key, 'v$key');
        expect(tree.isValid(), isTrue, reason: 'invalid after inserting $key');
      }

      final expected = List<int>.generate(50, (index) => index + 1);
      expect(tree.height, greaterThanOrEqualTo(2));
      expect(tree.fullScan().map((entry) => entry.$1), orderedEquals(expected));
      expect(tree.entries.map((entry) => entry.$1), orderedEquals(expected));
    });
  });

  group('range and ordered scans', () {
    test('range scans are inclusive and reject inverted bounds', () {
      final tree = BPlusTree<int, String>();
      for (final key in [9, 3, 7, 1, 5, 2, 8, 4, 6, 10]) {
        tree.insert(key, 'v$key');
      }

      expect(
        tree.rangeScan(3, 7).map((entry) => entry.$1),
        orderedEquals([3, 4, 5, 6, 7]),
      );
      expect(
        tree.rangeQuery(3, 7).map((entry) => entry.$1),
        orderedEquals([3, 4, 5, 6, 7]),
      );
      expect(tree.rangeScan(11, 20), isEmpty);
      expect(() => tree.rangeScan(7, 3), throwsArgumentError);
    });

    test('min, max, and empty edges are predictable', () {
      final tree = BPlusTree<int, String>();

      expect(() => tree.minKey(), throwsStateError);
      expect(() => tree.maxKey(), throwsStateError);
      expect(tree.fullScan(), isEmpty);
      expect(tree.inorder(), isEmpty);
      expect(tree.rangeScan(1, 10), isEmpty);

      tree
        ..insert(20, 'twenty')
        ..insert(10, 'ten')
        ..insert(30, 'thirty');
      expect(tree.minKey(), 10);
      expect(tree.maxKey(), 30);
      expect(tree.toString(), 'BPlusTree(t=2, size=3, height=0)');
    });
  });

  group('deletion and balancing', () {
    test('removes keys and treats missing deletion as a no-op', () {
      final tree = BPlusTree<int, String>();
      for (var key = 1; key <= 25; key++) {
        tree.insert(key, 'v$key');
      }

      for (final key in [7, 12, 1, 25, 13, 14, 15, 16, 17, 18]) {
        expect(tree.delete(key), isTrue);
        expect(tree.contains(key), isFalse);
        expect(tree.isValid(), isTrue, reason: 'invalid after deleting $key');
      }
      expect(tree.delete(99), isFalse);

      expect(tree.count, 15);
      expect(tree.minKey(), 2);
      expect(tree.maxKey(), 24);
      expect(tree.height, greaterThan(0));
    });

    test('all supported minimum degrees remain valid', () {
      for (final degree in [2, 3, 5, 8]) {
        final tree = BPlusTree<int, String>(degree);
        for (var key = 100; key >= 1; key--) {
          tree.insert(key, 'v$key');
        }
        expect(tree.isValid(), isTrue, reason: 'degree $degree');
        expect(
          tree.fullScan().map((entry) => entry.$1),
          orderedEquals(List<int>.generate(100, (index) => index + 1)),
        );
      }
    });

    test('deleting every key returns to the canonical empty tree', () {
      final tree = BPlusTree<int, String>(3);
      for (var key = 0; key < 80; key++) {
        tree.insert(key, 'v$key');
      }
      final keys = List<int>.generate(80, (index) => index)
        ..shuffle(Random(99));
      for (final key in keys) {
        expect(tree.delete(key), isTrue);
        expect(tree.isValid(), isTrue, reason: 'invalid after deleting $key');
      }

      expect(tree.isEmpty, isTrue);
      expect(tree.height, 0);
      expect(tree.fullScan(), isEmpty);
    });
  });

  test('randomized operations match a sorted reference map', () {
    final tree = BPlusTree<int, String?>(3);
    final reference = <int, String?>{};
    final random = Random(1234);

    for (var step = 0; step < 500; step++) {
      final key = random.nextInt(200);
      if (random.nextInt(4) == 0) {
        final wasPresent = reference.containsKey(key);
        reference.remove(key);
        expect(tree.delete(key), wasPresent);
      } else {
        final value = random.nextInt(5) == 0 ? null : 'v$step';
        tree.insert(key, value);
        reference[key] = value;
      }

      final expectedKeys = reference.keys.toList()..sort();
      expect(tree.count, reference.length);
      expect(tree.isValid(), isTrue, reason: 'invalid at step $step');
      expect(
        tree.fullScan().map((entry) => entry.$1),
        orderedEquals(expectedKeys),
      );
      for (final entry in reference.entries) {
        expect(tree.contains(entry.key), isTrue);
        expect(tree.search(entry.key), entry.value);
      }
    }
  });
}
