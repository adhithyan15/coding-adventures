import 'dart:math';

import 'package:coding_adventures_b_tree/coding_adventures_b_tree.dart';
import 'package:test/test.dart';

final class _Key {
  const _Key(this.value);

  final int value;
}

void main() {
  group('construction and metadata', () {
    test('rejects a minimum degree below two', () {
      expect(() => BTree<int, String>(1), throwsArgumentError);

      final tree = BTree<int, String>();
      expect(tree.minimumDegree, 2);
      expect(tree.count, 0);
      expect(tree.isEmpty, isTrue);
      expect(tree.height, 0);
      expect(tree.isValid(), isTrue);
      expect(tree.toString(), 'BTree(t=2, size=0, height=0)');
    });
  });

  group('insert and lookup', () {
    test('upserts values without changing count', () {
      final tree = BTree<int, String>(3);
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
      final tree = BTree<int, String?>()..insert(42, null);

      expect(tree.contains(42), isTrue);
      expect(tree.search(42), isNull);
      expect(tree.contains(7), isFalse);
      expect(tree.isValid(), isTrue);
    });

    test('accepts a custom comparator for non-Comparable keys', () {
      final tree = BTree<_Key, String>(
        2,
        (left, right) => left.value.compareTo(right.value),
      )
        ..insert(const _Key(2), 'two')
        ..insert(const _Key(1), 'one');

      expect(tree.search(const _Key(2)), 'two');
      expect(tree.inorder().map((entry) => entry.$1.value), [1, 2]);
      expect(tree.isValid(), isTrue);
    });

    test('sequential inserts split the root and traverse in order', () {
      final tree = BTree<int, String>();
      for (var key = 0; key < 100; key++) {
        tree.insert(key, 'v$key');
        expect(tree.isValid(), isTrue, reason: 'invalid after inserting $key');
      }

      expect(tree.count, 100);
      expect(tree.height, greaterThan(0));
      expect(
        tree.inorder().map((entry) => entry.$1),
        orderedEquals(List<int>.generate(100, (index) => index)),
      );
    });
  });

  group('deletion', () {
    test('handles leaf removal, borrowing, merging, and root shrink', () {
      final tree = BTree<int, String>();
      for (var key = 1; key <= 25; key++) {
        tree.insert(key, 'v$key');
      }

      const deletionOrder = [
        7,
        12,
        1,
        25,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        2,
        3,
        4,
        5,
        6,
        8,
        9,
        10,
      ];
      for (final key in deletionOrder) {
        expect(tree.delete(key), isTrue);
        expect(tree.contains(key), isFalse);
        expect(tree.isValid(), isTrue, reason: 'invalid after deleting $key');
      }

      expect(tree.count, 1);
      expect(tree.minKey(), 11);
      expect(tree.maxKey(), 11);
      expect(tree.height, 0);
    });

    test('missing deletion is a no-op', () {
      final tree = BTree<int, String>()..insert(10, 'ten');

      expect(tree.delete(99), isFalse);
      expect(tree.contains(10), isTrue);
      expect(tree.count, 1);
      expect(tree.isValid(), isTrue);
    });
  });

  group('ordered queries', () {
    test('min, max, and inclusive range queries cross tree levels', () {
      final tree = BTree<int, String>(3);
      for (var key = 1; key <= 50; key++) {
        tree.insert(key, 'v$key');
      }

      expect(tree.minKey(), 1);
      expect(tree.maxKey(), 50);
      expect(
        tree.rangeQuery(10, 20).map((entry) => entry.$1),
        orderedEquals(List<int>.generate(11, (index) => index + 10)),
      );
      expect(tree.rangeQuery(60, 70), isEmpty);
      expect(tree.rangeQuery(20, 10), isEmpty);
      expect(tree.isValid(), isTrue);
    });

    test('empty queries are predictable', () {
      final tree = BTree<int, String>();

      expect(() => tree.minKey(), throwsStateError);
      expect(() => tree.maxKey(), throwsStateError);
      expect(tree.rangeQuery(1, 10), isEmpty);
      expect(tree.inorder(), isEmpty);
    });
  });

  test('randomized operations match a sorted reference map', () {
    final tree = BTree<int, String>(3);
    final reference = <int, String>{};
    final random = Random(1234);
    final keys = List<int>.generate(400, (index) => index)..shuffle(random);

    for (final key in keys) {
      tree.insert(key, 'v$key');
      reference[key] = 'v$key';
    }
    for (final key in keys) {
      expect(tree.search(key), reference[key]);
    }
    for (final key in keys.take(175)) {
      expect(tree.delete(key), isTrue);
      reference.remove(key);
    }

    for (var step = 0; step < 100; step++) {
      final key = random.nextInt(600);
      if (random.nextBool()) {
        tree.insert(key, 'v$key');
        reference[key] = 'v$key';
      } else {
        final wasPresent = reference.remove(key) != null;
        expect(tree.delete(key), wasPresent);
      }
    }

    final expectedKeys = reference.keys.toList()..sort();
    expect(tree.count, reference.length);
    expect(tree.isValid(), isTrue);
    expect(
      tree.inorder().map((entry) => entry.$1),
      orderedEquals(expectedKeys),
    );
  });
}
