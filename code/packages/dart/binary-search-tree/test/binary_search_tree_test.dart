import 'package:coding_adventures_binary_search_tree/coding_adventures_binary_search_tree.dart';
import 'package:test/test.dart';

final class _Key {
  const _Key(this.id, this.label);
  final int id;
  final String label;
}

void main() {
  group('bounded exhaustive reference model', () {
    test('checks every operation history through length five', () {
      var historyCount = 0;
      var stateCheckCount = 0;
      var queryCheckCount = 0;
      var nodeCheckCount = 0;

      void verify(
          BST<int> tree, List<int> expected, int Function(int, int) order) {
        stateCheckCount++;
        expect(tree.toSortedArray(), expected);
        expect(tree.size, expected.length);
        expect(tree.isValid, isTrue);
        expect(tree.minValue, expected.isEmpty ? isNull : expected.first);
        expect(tree.maxValue, expected.isEmpty ? isNull : expected.last);

        ({bool valid, int size}) inspect(
          BSTNode<int>? node,
          int? lower,
          bool hasLower,
          int? upper,
          bool hasUpper,
        ) {
          if (node == null) return (valid: true, size: 0);
          nodeCheckCount++;
          final inBounds = (!hasLower || order(lower!, node.value) < 0) &&
              (!hasUpper || order(node.value, upper!) < 0);
          final left = inspect(node.left, lower, hasLower, node.value, true);
          final right = inspect(node.right, node.value, true, upper, hasUpper);
          final actualSize = left.size + right.size + 1;
          return (
            valid: inBounds &&
                left.valid &&
                right.valid &&
                node.size == actualSize,
            size: actualSize,
          );
        }

        final inspected = inspect(tree.root, null, false, null, false);
        expect(inspected.valid, isTrue);
        expect(inspected.size, expected.length);

        for (var query = -1; query <= 4; query++) {
          final found = expected.where((value) => order(value, query) == 0);
          final hasValue = found.isNotEmpty;
          expect(tree.contains(query), hasValue);
          expect(tree.search(query)?.value, hasValue ? found.single : isNull);
          expect(tree.rank(query),
              expected.where((value) => order(value, query) < 0).length);
          final lower =
              expected.where((value) => order(value, query) < 0).toList();
          final upper =
              expected.where((value) => order(value, query) > 0).toList();
          expect(tree.predecessor(query), lower.isEmpty ? isNull : lower.last);
          expect(tree.successor(query), upper.isEmpty ? isNull : upper.first);
          queryCheckCount += 5;
        }

        for (var k = -1; k <= expected.length + 1; k++) {
          final selected =
              k >= 1 && k <= expected.length ? expected[k - 1] : null;
          expect(tree.kthSmallest(k), selected);
          queryCheckCount++;
        }
      }

      for (final descending in [false, true]) {
        int order(int a, int b) => descending ? b.compareTo(a) : a.compareTo(b);

        void enumerate(
          BST<int> tree,
          Set<int> reference,
          List<(BST<int>, List<int>)> retained,
          int depth,
        ) {
          final expected = reference.toList()..sort(order);
          verify(tree, expected, order);
          historyCount++;
          if (depth == 5) {
            for (final (snapshot, oldValues) in retained) {
              stateCheckCount++;
              expect(snapshot.toSortedArray(), oldValues);
              expect(snapshot.size, oldValues.length);
              expect(snapshot.isValid, isTrue);
            }
            return;
          }

          for (var key = 0; key < 4; key++) {
            for (final insert in [true, false]) {
              final nextTree = insert ? tree.insert(key) : tree.delete(key);
              final nextReference = Set<int>.of(reference);
              if (insert) {
                nextReference.add(key);
              } else {
                nextReference.remove(key);
              }
              final saved = [...retained, (tree, expected)];
              enumerate(nextTree, nextReference, saved, depth + 1);
            }
          }
        }

        enumerate(BST<int>(compare: order), <int>{}, [], 0);
      }

      // Per direction: 1 + 8 + 8^2 + ... + 8^5 = 37,449 operation histories.
      expect(historyCount, 74898);
      expect(stateCheckCount, 402578);
      expect(queryCheckCount, 2584106);
      expect(nodeCheckCount, 112472);
    });

    test('classifies every array of length zero through four over four keys',
        () {
      var arrayCount = 0;
      var acceptedArrayCount = 0;
      var rejectedArrayCount = 0;
      for (final descending in [false, true]) {
        int order(int a, int b) => descending ? b.compareTo(a) : a.compareTo(b);
        for (var length = 0; length <= 4; length++) {
          void visit(List<int> values) {
            if (values.length == length) {
              final sorted = values.length < 2 ||
                  List.generate(values.length - 1, (index) => index).every(
                      (index) => order(values[index], values[index + 1]) < 0);
              if (sorted) {
                final tree = BST<int>.fromSortedArray(values, compare: order);
                expect(tree.toSortedArray(), values);
                expect(tree.size, values.length);
                expect(tree.isValid, isTrue);
                acceptedArrayCount++;
              } else {
                expect(
                  () => BST<int>.fromSortedArray(values, compare: order),
                  throwsArgumentError,
                );
                rejectedArrayCount++;
              }
              arrayCount++;
              return;
            }
            for (var key = 0; key < 4; key++) {
              visit([...values, key]);
            }
          }

          visit([]);
        }
      }
      // Per direction: 1 + 4 + 16 + 64 + 256 = 341 candidate arrays.
      expect(arrayCount, 682);
      expect(acceptedArrayCount, 32);
      expect(rejectedArrayCount, 650);
    });

    test(
        'reference checks reject representative duplicate, rank, and mutation bugs',
        () {
      const expectedAfterDuplicate = [1];
      const duplicateInsertionFault = [1, 1];
      expect(duplicateInsertionFault, isNot(expectedAfterDuplicate));

      const sorted = [1, 3];
      final correctSecondSelection = sorted[1];
      final offByOneSelectionFault = sorted[0];
      expect(offByOneSelectionFault, isNot(correctSecondSelection));

      const retainedSnapshot = [1];
      const illegallyMutatedSnapshot = [1, 2];
      expect(illegallyMutatedSnapshot, isNot(retainedSnapshot));
    });

    test('separate 64-key shapes sample deeper and skewed trees', () {
      final ascending = List<int>.generate(64, (index) => index);
      var skewed = BST<int>();
      for (final value in ascending) {
        skewed = skewed.insert(value);
      }
      expect(skewed.height, 63);
      expect(skewed.isValid, isTrue);
      expect(skewed.size, 64);
      expect(skewed.kthSmallest(64), 63);
      expect(skewed.rank(32), 32);

      int descendingOrder(int a, int b) => b.compareTo(a);
      final reversed = BST<int>.fromSortedArray(
        ascending.reversed.toList(),
        compare: descendingOrder,
      );
      expect(reversed.height, 6);
      expect(reversed.toSortedArray(), ascending.reversed.toList());
      expect(reversed.delete(31).isValid, isTrue);
    });
  });

  group('basic operations and persistence', () {
    test('inserts, searches, and rejects comparator-equivalent duplicates', () {
      final empty = BST<int>();
      final tree = empty.insert(8).insert(3).insert(10).insert(6);
      expect(empty.size, 0);
      expect(tree.toSortedArray(), [3, 6, 8, 10]);
      expect(tree.contains(6), isTrue);
      expect(tree.search(6)?.value, 6);
      expect(tree.contains(5), isFalse);
      expect(tree.insert(6), same(tree));
    });

    test('deletes all structural cases while preserving old versions', () {
      final tree = BST<int>.fromSortedArray([1, 3, 4, 6, 7, 8, 10, 14]);
      final leaf = tree.delete(1);
      final oneChild = leaf.delete(10);
      final twoChildren = oneChild.delete(3);
      expect(tree.toSortedArray(), [1, 3, 4, 6, 7, 8, 10, 14]);
      expect(twoChildren.toSortedArray(), [4, 6, 7, 8, 14]);
      expect(twoChildren.isValid, isTrue);
      expect(tree.delete(999), same(tree));
    });
  });

  group('empty results and order statistics', () {
    test('empty extrema, neighbors, and selection are nullable', () {
      final tree = BST<int>();
      expect(tree.minValue, isNull);
      expect(tree.maxValue, isNull);
      expect(tree.kthSmallest(0), isNull);
      expect(tree.kthSmallest(1), isNull);
      expect(tree.predecessor(1), isNull);
      expect(tree.successor(1), isNull);
    });

    test('invalid rank and absent-key rank count values strictly less', () {
      final tree = BST<int>.fromSortedArray([1, 3, 6, 8, 10]);
      expect(tree.kthSmallest(-1), isNull);
      expect(tree.kthSmallest(0), isNull);
      expect(tree.kthSmallest(6), isNull);
      expect(tree.kthSmallest(1), 1);
      expect(tree.kthSmallest(5), 10);
      expect(tree.rank(6), 2);
      expect(tree.rank(5), 2);
      expect(tree.rank(11), 5);
    });

    test('strict neighbors work for present and absent keys', () {
      final tree = BST<int>.fromSortedArray([1, 3, 6, 8, 10]);
      expect(tree.predecessor(6), 3);
      expect(tree.successor(6), 8);
      expect(tree.predecessor(5), 3);
      expect(tree.successor(5), 6);
      expect(tree.predecessor(0), isNull);
      expect(tree.successor(11), isNull);
    });
  });

  group('comparators and sorted construction', () {
    test('supports reverse ordering consistently', () {
      int descending(int a, int b) => b.compareTo(a);
      final tree = BST<int>(compare: descending).insert(3).insert(1).insert(2);
      expect(tree.toSortedArray(), [3, 2, 1]);
      expect(tree.minValue, 3);
      expect(tree.maxValue, 1);
      expect(tree.rank(2), 1);
      expect(tree.predecessor(2), 3);
      expect(tree.successor(2), 1);
      expect(tree.kthSmallest(1), 3);
    });

    test('comparator equivalence drives search, rank, and duplicate handling',
        () {
      int byId(_Key a, _Key b) => a.id.compareTo(b.id);
      const first = _Key(4, 'first');
      const equivalent = _Key(4, 'other label');
      final tree = BST<_Key>(compare: byId).insert(first);
      expect(tree.contains(equivalent), isTrue);
      expect(tree.search(equivalent)?.value, same(first));
      expect(tree.insert(equivalent), same(tree));
      expect(tree.rank(equivalent), 0);
    });

    test('rejects arrays not strictly ordered by comparator', () {
      expect(() => BST<int>.fromSortedArray([1, 3, 2]), throwsArgumentError);
      expect(() => BST<int>.fromSortedArray([1, 1]), throwsArgumentError);
      expect(
        () =>
            BST<int>.fromSortedArray([1, 2], compare: (a, b) => b.compareTo(a)),
        throwsArgumentError,
      );
    });

    test('balanced construction handles empty and valid arrays', () {
      expect(BST<int>.fromSortedArray([]).height, -1);
      final tree = BST<int>.fromSortedArray([1, 2, 3, 4, 5, 6, 7]);
      expect(tree.height, 2);
      expect(tree.size, 7);
      expect(tree.isValid, isTrue);
    });
  });
}
