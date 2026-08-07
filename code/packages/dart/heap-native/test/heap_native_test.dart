import 'package:coding_adventures_heap_native/coding_adventures_heap_native.dart';
import 'package:test/test.dart';

/// Exercises the Rust heaps through dart:ffi, matching the pure-Dart port.
void main() {
  group('MinHeap (native)', () {
    test('pops ascending', () {
      final h = MinHeap();
      for (final v in [5, 3, 8, 1, 9, 2, 7]) {
        h.push(v);
      }
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([1, 2, 3, 5, 7, 8, 9]));
      h.dispose();
    });

    test('peek / length / empty', () {
      final h = MinHeap()..push(4)..push(2)..push(6);
      expect(h.peek(), equals(2));
      expect(h.length, equals(3));
      expect(h.isEmpty, isFalse);
      h.dispose();
    });

    test('pop and peek on empty return null', () {
      final h = MinHeap();
      expect(h.isEmpty, isTrue);
      expect(h.pop(), isNull);
      expect(h.peek(), isNull);
      h.dispose();
    });

    test('handles negative values and duplicates', () {
      final h = MinHeap();
      for (final v in [-3, 5, -3, 0, 5, -10]) {
        h.push(v);
      }
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([-10, -3, -3, 0, 5, 5]));
      h.dispose();
    });

    test('using a disposed heap throws', () {
      final h = MinHeap()..push(1);
      h.dispose();
      expect(() => h.push(2), throwsStateError);
      expect(() => h.pop(), throwsStateError);
      h.dispose(); // idempotent
    });
  });

  group('MaxHeap (native)', () {
    test('pops descending', () {
      final h = MaxHeap();
      for (final v in [5, 3, 8, 1, 9, 2, 7]) {
        h.push(v);
      }
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([9, 8, 7, 5, 3, 2, 1]));
      h.dispose();
    });
  });

  group('array algorithms (native)', () {
    test('heapSort sorts ascending', () {
      expect(heapSort([3, 1, 4, 1, 5, 9, 2, 6]),
          equals([1, 1, 2, 3, 4, 5, 6, 9]));
      expect(heapSort(<int>[]), isEmpty);
      expect(heapSort([7]), equals([7]));
    });

    test('nLargest descending, with n bounds', () {
      expect(nLargest([5, 1, 4, 2, 3], 2), equals([5, 4]));
      expect(nLargest([3, 1, 2], 0), isEmpty);
      expect(nLargest([3, 1, 2], 9), equals([3, 2, 1]));
    });

    test('nSmallest ascending, with n bounds', () {
      expect(nSmallest([5, 1, 4, 2, 3], 2), equals([1, 2]));
      expect(nSmallest([3, 1, 2], 0), isEmpty);
      expect(nSmallest([3, 1, 2], 9), equals([1, 2, 3]));
    });

    test('matches manual sorts on random-ish data', () {
      final data = [42, 7, 19, 88, 3, 61, 25, 88, 7];
      final asc = [...data]..sort();
      final desc = [...data]..sort((a, b) => b.compareTo(a));
      expect(heapSort(data), equals(asc));
      expect(nLargest(data, 4), equals(desc.take(4).toList()));
      expect(nSmallest(data, 4), equals(asc.take(4).toList()));
    });
  });
}
