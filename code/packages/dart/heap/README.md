# coding_adventures_heap

Array-backed binary heaps and companion algorithms, in pure Dart.

Dart port of the `heap` package that already exists in Rust, Java, Kotlin, and
other languages in the monorepo.

## What it provides

| Type / function | Purpose |
|---|---|
| `MinHeap<T>` | Binary heap with the smallest element at the root. |
| `MaxHeap<T>` | Binary heap with the largest element at the root. |
| `heapify(items)` | Heap-ordered array (as a `MinHeap` stores it). |
| `heapSort(items)` | Ascending sort via repeated min-heap pops. |
| `nLargest(items, n)` | The `n` largest, descending — bounded-heap, O(m log n). |
| `nSmallest(items, n)` | The `n` smallest, ascending. |

Heaps support `push`, `pop`, `peek`, `isEmpty`, `length`, `toList`, and a
`fromIterable` constructor that heapifies in O(n).

## Ordering

Ordering defaults to the natural order of the elements (`int`, `double`,
`String`, …). Pass a `Comparator<T>` to order by any key:

```dart
// Min-heap keyed by string length:
final h = MinHeap<String>((a, b) => a.length.compareTo(b.length));
```

## Usage

```dart
import 'package:coding_adventures_heap/coding_adventures_heap.dart';

void main() {
  final h = MinHeap<int>()..push(5)..push(1)..push(3);
  print(h.pop()); // 1
  print(h.pop()); // 3

  print(heapSort([3, 1, 2]));           // [1, 2, 3]
  print(nLargest([5, 1, 4, 2, 3], 2));  // [5, 4]
  print(nSmallest([5, 1, 4, 2, 3], 2)); // [1, 2]
}
```

## Running the tests

```
dart pub get
dart test
```
