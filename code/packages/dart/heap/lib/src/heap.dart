/// Array-backed binary heaps and companion algorithms, in pure Dart.
///
/// A binary heap is a complete binary tree stored in a flat list: the children
/// of index `i` live at `2i+1` and `2i+2`. A [MinHeap] keeps the smallest
/// element at the root; a [MaxHeap] keeps the largest. Both give O(log n) push
/// and pop and O(1) peek.
///
/// Ordering defaults to the natural order of the elements (via
/// [Comparable.compare]), so `int`, `double`, and `String` work out of the box.
/// Pass a custom [Comparator] to order by any key.
library heap;

/// A comparator that orders elements by their natural [Comparable] order.
Comparator<T> _naturalOrder<T>() =>
    (a, b) => Comparable.compare(a as Comparable, b as Comparable);

// ─── Internal sift/build helpers ─────────────────────────────────────────────
//
// All three take a `higherPriority(a, b)` predicate — "should a sit above b?".
// For a min-heap that is `a < b`; for a max-heap `a > b`. Everything else about
// the heap machinery is identical, so the two heap types share this code.

void _siftUp<T>(List<T> data, int index, bool Function(T, T) higherPriority) {
  while (index > 0) {
    final parent = (index - 1) ~/ 2;
    if (higherPriority(data[index], data[parent])) {
      final tmp = data[index];
      data[index] = data[parent];
      data[parent] = tmp;
      index = parent;
    } else {
      break;
    }
  }
}

void _siftDown<T>(List<T> data, int index, bool Function(T, T) higherPriority) {
  final len = data.length;
  while (true) {
    final left = 2 * index + 1;
    final right = 2 * index + 2;
    var best = index;
    if (left < len && higherPriority(data[left], data[best])) best = left;
    if (right < len && higherPriority(data[right], data[best])) best = right;
    if (best == index) break;
    final tmp = data[index];
    data[index] = data[best];
    data[best] = tmp;
    index = best;
  }
}

void _buildHeap<T>(List<T> data, bool Function(T, T) higherPriority) {
  if (data.length < 2) return;
  for (var index = (data.length - 2) ~/ 2; index >= 0; index--) {
    _siftDown(data, index, higherPriority);
  }
}

/// Shared implementation of both heaps, parameterised by a priority predicate.
abstract class _Heap<T> {
  final List<T> _data = <T>[];
  final Comparator<T> _compare;

  _Heap(Comparator<T>? compare) : _compare = compare ?? _naturalOrder<T>();

  /// True when [a] should sit above [b] in this heap. Subclasses pick the sign.
  bool _higherPriority(T a, T b);

  void _fill(Iterable<T> items) {
    _data.addAll(items);
    _buildHeap(_data, _higherPriority);
  }

  /// Insert [value], restoring the heap property in O(log n).
  void push(T value) {
    _data.add(value);
    _siftUp(_data, _data.length - 1, _higherPriority);
  }

  /// Remove and return the root (smallest for [MinHeap], largest for [MaxHeap]),
  /// or `null` when the heap is empty.
  T? pop() {
    if (_data.isEmpty) return null;
    final last = _data.removeLast();
    if (_data.isEmpty) return last;
    final root = _data[0];
    _data[0] = last;
    _siftDown(_data, 0, _higherPriority);
    return root;
  }

  /// The root element without removing it, or `null` when empty.
  T? peek() => _data.isEmpty ? null : _data[0];

  /// True when the heap has no elements.
  bool get isEmpty => _data.isEmpty;

  /// The number of elements in the heap.
  int get length => _data.length;

  /// A copy of the underlying array (heap order, not sorted).
  List<T> toList() => List<T>.of(_data);
}

/// A binary heap whose root is always the **smallest** element.
class MinHeap<T> extends _Heap<T> {
  /// Create an empty min-heap ordered by [compare] (natural order if omitted).
  MinHeap([Comparator<T>? compare]) : super(compare);

  /// Build a min-heap from [items] in O(n) via bottom-up heapification.
  factory MinHeap.fromIterable(Iterable<T> items, [Comparator<T>? compare]) {
    final h = MinHeap<T>(compare);
    h._fill(items);
    return h;
  }

  @override
  bool _higherPriority(T a, T b) => _compare(a, b) < 0;

  @override
  String toString() => isEmpty
      ? 'MinHeap(size=0, root=empty)'
      : 'MinHeap(size=$length, root=${peek()})';
}

/// A binary heap whose root is always the **largest** element.
class MaxHeap<T> extends _Heap<T> {
  /// Create an empty max-heap ordered by [compare] (natural order if omitted).
  MaxHeap([Comparator<T>? compare]) : super(compare);

  /// Build a max-heap from [items] in O(n) via bottom-up heapification.
  factory MaxHeap.fromIterable(Iterable<T> items, [Comparator<T>? compare]) {
    final h = MaxHeap<T>(compare);
    h._fill(items);
    return h;
  }

  @override
  bool _higherPriority(T a, T b) => _compare(a, b) > 0;

  @override
  String toString() => isEmpty
      ? 'MaxHeap(size=0, root=empty)'
      : 'MaxHeap(size=$length, root=${peek()})';
}

// ─── Companion algorithms ────────────────────────────────────────────────────

/// Return the heap-ordered array of [items] (as a [MinHeap] would store them).
List<T> heapify<T>(Iterable<T> items, [Comparator<T>? compare]) =>
    MinHeap<T>.fromIterable(items, compare).toList();

/// Return [items] sorted ascending, by repeatedly popping a min-heap
/// (heapsort, O(n log n)).
List<T> heapSort<T>(Iterable<T> items, [Comparator<T>? compare]) {
  final heap = MinHeap<T>.fromIterable(items, compare);
  final result = <T>[];
  while (!heap.isEmpty) {
    result.add(heap.pop() as T);
  }
  return result;
}

/// Return the [n] largest elements of [iterable], in descending order.
///
/// Uses a bounded min-heap of size `n`, so it is O(m log n) for `m` items —
/// much cheaper than a full sort when `n ≪ m`.
List<T> nLargest<T>(Iterable<T> iterable, int n, [Comparator<T>? compare]) {
  final cmp = compare ?? _naturalOrder<T>();
  if (n <= 0) return <T>[];
  final items = List<T>.of(iterable);
  if (n >= items.length) {
    items.sort((a, b) => cmp(b, a)); // descending
    return items;
  }
  final heap = MinHeap<T>.fromIterable(items.sublist(0, n), cmp);
  for (final value in items.sublist(n)) {
    final top = heap.peek();
    if (top == null || cmp(value, top) > 0) {
      heap.pop();
      heap.push(value);
    }
  }
  final result = <T>[];
  while (!heap.isEmpty) {
    result.add(heap.pop() as T);
  }
  return result.reversed.toList();
}

/// Return the [n] smallest elements of [iterable], in ascending order.
///
/// Uses a bounded max-heap of size `n` (the mirror of [nLargest]).
List<T> nSmallest<T>(Iterable<T> iterable, int n, [Comparator<T>? compare]) {
  final cmp = compare ?? _naturalOrder<T>();
  if (n <= 0) return <T>[];
  final items = List<T>.of(iterable);
  if (n >= items.length) {
    items.sort(cmp); // ascending
    return items;
  }
  final heap = MaxHeap<T>.fromIterable(items.sublist(0, n), cmp);
  for (final value in items.sublist(n)) {
    final top = heap.peek();
    if (top == null || cmp(value, top) < 0) {
      heap.pop();
      heap.push(value);
    }
  }
  final result = <T>[];
  while (!heap.isEmpty) {
    result.add(heap.pop() as T);
  }
  return result.reversed.toList();
}
