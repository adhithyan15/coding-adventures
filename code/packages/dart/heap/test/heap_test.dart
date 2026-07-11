import 'package:coding_adventures_heap/coding_adventures_heap.dart';
import 'package:test/test.dart';

/// Validate that [values] satisfies the min-heap property.
bool isValidMinHeap(List<int> values) {
  for (var i = 0; i < values.length; i++) {
    final l = 2 * i + 1, r = 2 * i + 2;
    if (l < values.length && values[i] > values[l]) return false;
    if (r < values.length && values[i] > values[r]) return false;
  }
  return true;
}

void main() {
  // ==========================================================================
  // MinHeap
  // ==========================================================================
  group('MinHeap', () {
    test('pops in ascending order', () {
      final h = MinHeap<int>();
      for (final v in [5, 3, 8, 1, 9, 2, 7]) {
        h.push(v);
      }
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([1, 2, 3, 5, 7, 8, 9]));
    });

    test('peek returns the smallest without removing it', () {
      final h = MinHeap<int>()..push(4)..push(2)..push(6);
      expect(h.peek(), equals(2));
      expect(h.length, equals(3));
    });

    test('pop and peek on empty return null', () {
      final h = MinHeap<int>();
      expect(h.isEmpty, isTrue);
      expect(h.pop(), isNull);
      expect(h.peek(), isNull);
    });

    test('single element', () {
      final h = MinHeap<int>()..push(42);
      expect(h.pop(), equals(42));
      expect(h.isEmpty, isTrue);
    });

    test('fromIterable builds a valid heap with the min at the root', () {
      final h = MinHeap<int>.fromIterable([9, 4, 7, 1, 8, 2, 6, 3, 5]);
      expect(h.peek(), equals(1));
      expect(isValidMinHeap(h.toList()), isTrue);
      expect(h.length, equals(9));
    });

    test('handles duplicates', () {
      final h = MinHeap<int>.fromIterable([3, 1, 3, 1, 2, 2]);
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([1, 1, 2, 2, 3, 3]));
    });

    test('toString reports size and root', () {
      expect(MinHeap<int>().toString(), equals('MinHeap(size=0, root=empty)'));
      expect((MinHeap<int>()..push(7)).toString(),
          equals('MinHeap(size=1, root=7)'));
    });
  });

  // ==========================================================================
  // MaxHeap
  // ==========================================================================
  group('MaxHeap', () {
    test('pops in descending order', () {
      final h = MaxHeap<int>();
      for (final v in [5, 3, 8, 1, 9, 2, 7]) {
        h.push(v);
      }
      final out = <int>[];
      while (!h.isEmpty) {
        out.add(h.pop()!);
      }
      expect(out, equals([9, 8, 7, 5, 3, 2, 1]));
    });

    test('fromIterable puts the max at the root', () {
      final h = MaxHeap<int>.fromIterable([9, 4, 7, 1, 8]);
      expect(h.peek(), equals(9));
    });
  });

  // ==========================================================================
  // Custom comparator & other element types
  // ==========================================================================
  group('ordering', () {
    test('works with strings (natural order)', () {
      final h = MinHeap<String>.fromIterable(['pear', 'apple', 'fig']);
      expect(h.pop(), equals('apple'));
      expect(h.pop(), equals('fig'));
    });

    test('custom comparator: min-heap by string length', () {
      final h = MinHeap<String>((a, b) => a.length.compareTo(b.length))
        ..push('ccc')
        ..push('a')
        ..push('bb');
      expect(h.pop(), equals('a'));
      expect(h.pop(), equals('bb'));
      expect(h.pop(), equals('ccc'));
    });
  });

  // ==========================================================================
  // Companion algorithms
  // ==========================================================================
  group('heapify', () {
    test('produces a valid min-heap array', () {
      final arr = heapify([9, 4, 7, 1, 8, 2]);
      expect(isValidMinHeap(arr), isTrue);
      expect(arr.length, equals(6));
      expect(arr.first, equals(1));
    });
    test('empty input', () {
      expect(heapify(<int>[]), isEmpty);
    });
  });

  group('heapSort', () {
    test('sorts ascending', () {
      expect(heapSort([3, 1, 4, 1, 5, 9, 2, 6]), equals([1, 1, 2, 3, 4, 5, 6, 9]));
    });
    test('empty and single', () {
      expect(heapSort(<int>[]), isEmpty);
      expect(heapSort([7]), equals([7]));
    });
    test('already sorted and reversed', () {
      expect(heapSort([1, 2, 3, 4]), equals([1, 2, 3, 4]));
      expect(heapSort([4, 3, 2, 1]), equals([1, 2, 3, 4]));
    });
  });

  group('nLargest', () {
    test('returns the n largest in descending order', () {
      expect(nLargest([5, 1, 4, 2, 3], 2), equals([5, 4]));
      expect(nLargest([5, 1, 4, 2, 3], 3), equals([5, 4, 3]));
    });
    test('n == 0 gives empty, n >= length gives full descending sort', () {
      expect(nLargest([3, 1, 2], 0), isEmpty);
      expect(nLargest([3, 1, 2], 5), equals([3, 2, 1]));
    });
    test('matches a manual sort for random-ish data', () {
      final data = [42, 7, 19, 88, 3, 61, 25, 88, 7];
      final sortedDesc = [...data]..sort((a, b) => b.compareTo(a));
      expect(nLargest(data, 4), equals(sortedDesc.take(4).toList()));
    });
  });

  group('nSmallest', () {
    test('returns the n smallest in ascending order', () {
      expect(nSmallest([5, 1, 4, 2, 3], 2), equals([1, 2]));
      expect(nSmallest([5, 1, 4, 2, 3], 3), equals([1, 2, 3]));
    });
    test('n == 0 gives empty, n >= length gives full ascending sort', () {
      expect(nSmallest([3, 1, 2], 0), isEmpty);
      expect(nSmallest([3, 1, 2], 5), equals([1, 2, 3]));
    });
    test('matches a manual sort for random-ish data', () {
      final data = [42, 7, 19, 88, 3, 61, 25, 88, 7];
      final sortedAsc = [...data]..sort();
      expect(nSmallest(data, 4), equals(sortedAsc.take(4).toList()));
    });
  });
}
