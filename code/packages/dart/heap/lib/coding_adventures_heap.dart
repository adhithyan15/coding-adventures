/// Array-backed binary heaps ([MinHeap], [MaxHeap]) and companion algorithms
/// ([heapify], [heapSort], [nLargest], [nSmallest]), in pure Dart.
///
/// Ordering defaults to natural order (`int`, `double`, `String`, …); pass a
/// [Comparator] to order by any key.
///
/// ```dart
/// import 'package:coding_adventures_heap/coding_adventures_heap.dart';
///
/// void main() {
///   final h = MinHeap<int>()..push(5)..push(1)..push(3);
///   print(h.pop()); // 1
///   print(h.pop()); // 3
///
///   print(heapSort([3, 1, 2]));        // [1, 2, 3]
///   print(nLargest([5, 1, 4, 2, 3], 2)); // [5, 4]
/// }
/// ```
library coding_adventures_heap;

export 'src/heap.dart';
