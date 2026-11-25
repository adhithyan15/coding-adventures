/// Ordered sets over balanced-tree backends: the Dart lane.
///
/// A [TreeSet] is an immutable sorted set: ordered iteration, rank access,
/// range slices, and full set algebra, all backed by a pluggable
/// [OrderedTreeBackend] (AVL by default — the first consumer of the
/// in-lane `avl-tree` package).
///
/// ```dart
/// import 'package:coding_adventures_tree_set/coding_adventures_tree_set.dart';
///
/// void main() {
///   final a = TreeSet<int>.fromList([3, 1, 2]);
///   final b = TreeSet<int>.fromList([2, 4]);
///   print(a.union(b).toSortedArray()); // [1, 2, 3, 4]
///   print(a.intersection(b).toSortedArray()); // [2]
/// }
/// ```
library tree_set;

import 'package:coding_adventures_avl_tree/coding_adventures_avl_tree.dart'
    as avl;

// ─── Backend Contract ───────────────────────────────────────────────────────
//
// The operations a balanced tree must supply for set duty. Mirrors the
// reference trait; the AVL adapter below is the default (and currently only)
// implementation. A future red-black lane plugs in by implementing this.

/// Balanced-tree operations backing a [TreeSet].
abstract class OrderedTreeBackend<T extends Comparable<dynamic>> {
  /// Returns a backend holding [value] (no-op if present).
  OrderedTreeBackend<T> insert(T value);

  /// Returns a backend without [value] (no-op if absent).
  OrderedTreeBackend<T> delete(T value);

  /// True when [value] is present.
  bool contains(T value);

  /// Smallest value, or null when empty.
  T? minValue();

  /// Largest value, or null when empty.
  T? maxValue();

  /// Greatest value strictly below [value], or null.
  T? predecessor(T value);

  /// Smallest value strictly above [value], or null.
  T? successor(T value);

  /// 1-based k-th smallest value, or null when out of range.
  T? kthSmallest(int k);

  /// All values in sorted order.
  List<T> toSortedArray();

  /// Number of values stored.
  int size();

  /// Count of values strictly below [value] (0-based rank).
  int rank(T value);
}

/// [OrderedTreeBackend] implemented over the in-lane AVL tree.
class AvlTreeBackend<T extends Comparable<dynamic>>
    implements OrderedTreeBackend<T> {
  final avl.AvlTree<T> _tree;

  /// Wraps an existing tree (defaults to empty).
  AvlTreeBackend([avl.AvlTree<T>? tree])
      : _tree = tree ?? avl.AvlTree<T>.empty();

  @override
  AvlTreeBackend<T> insert(T value) => AvlTreeBackend(_tree.insert(value));

  @override
  AvlTreeBackend<T> delete(T value) => AvlTreeBackend(_tree.delete(value));

  @override
  bool contains(T value) => _tree.contains(value);

  @override
  T? minValue() => _tree.minValue();

  @override
  T? maxValue() => _tree.maxValue();

  @override
  T? predecessor(T value) => _tree.predecessor(value);

  @override
  T? successor(T value) => _tree.successor(value);

  @override
  T? kthSmallest(int k) => _tree.kthSmallest(k);

  @override
  List<T> toSortedArray() => _tree.toSortedArray();

  @override
  int size() => _tree.size();

  @override
  int rank(T value) => _tree.rank(value);
}

// ─── TreeSet ────────────────────────────────────────────────────────────────

/// An immutable ordered set over a swappable tree backend (AVL default).
class TreeSet<T extends Comparable<dynamic>> {
  final OrderedTreeBackend<T> _backend;

  const TreeSet._(this._backend);

  /// Creates an empty set on the default AVL backend.
  factory TreeSet.empty() => TreeSet._(AvlTreeBackend<T>());

  /// Creates an empty set (alias of [TreeSet.empty]).
  factory TreeSet() => TreeSet<T>.empty();

  /// Wraps an explicit [backend].
  factory TreeSet.withBackend(OrderedTreeBackend<T> backend) =>
      TreeSet._(backend);

  /// The backing tree.
  OrderedTreeBackend<T> get backend => _backend;

  /// Builds a set from [values] in order.
  factory TreeSet.fromList(Iterable<T> values) {
    var set = TreeSet<T>.empty();
    for (final value in values) {
      set = set.insert(value);
    }
    return set;
  }

  /// Builds a set from values (alias kept for cross-lane parity).
  factory TreeSet.fromSortedArray(Iterable<T> values) =>
      TreeSet<T>.fromList(values);

  /// Number of values stored.
  int get length => _backend.size();

  /// Alias of [length] kept for cross-lane parity.
  int size() => length;

  /// True when no values are stored.
  bool get isEmpty => length == 0;

  /// True when at least one value is stored.
  bool get isNotEmpty => length != 0;

  /// True when [value] is present.
  bool contains(T value) => _backend.contains(value);

  /// Smallest value, or null when empty.
  T? minValue() => _backend.minValue();

  /// Largest value, or null when empty.
  T? maxValue() => _backend.maxValue();

  /// Smallest value (alias kept for cross-lane parity).
  T? first() => minValue();

  /// Largest value (alias kept for cross-lane parity).
  T? last() => maxValue();

  /// Greatest value strictly below [value], or null.
  T? predecessor(T value) => _backend.predecessor(value);

  /// Smallest value strictly above [value], or null.
  T? successor(T value) => _backend.successor(value);

  /// 1-based k-th smallest value, or null when out of range.
  T? kthSmallest(int k) => _backend.kthSmallest(k);

  /// Count of values strictly below [value] (0-based rank).
  int rank(T value) => _backend.rank(value);

  /// All values in sorted order.
  List<T> toSortedArray() => _backend.toSortedArray();

  /// All values in sorted order (alias kept for cross-lane parity).
  List<T> toList() => toSortedArray();

  /// Sorted values as an iterable.
  Iterable<T> get keys => toSortedArray();

  /// Returns a set with [value] added (no-op if present).
  TreeSet<T> insert(T value) => TreeSet._(_backend.insert(value));

  /// Returns a set without [value] (no-op if absent).
  TreeSet<T> remove(T value) => TreeSet._(_backend.delete(value));

  /// Alias of [remove] kept for cross-lane parity.
  TreeSet<T> delete(T value) => remove(value);

  /// Values in `[min, max]` (or strict bounds when not [inclusive]).
  ///
  /// Empty when `min > max`. Binary-searched slice boundaries keep the
  /// scan to the result window.
  List<T> range(T min, T max, bool inclusive) {
    if (min.compareTo(max) > 0) return [];
    final values = toSortedArray();
    final start = inclusive
        ? _partitionPoint(values, min, false)
        : _partitionPoint(values, min, true);
    final end = inclusive
        ? _partitionPoint(values, max, true)
        : _partitionPoint(values, max, false);
    if (start >= end) return [];
    return values.sublist(start, end);
  }

  /// Union of both sets, in sorted order.
  TreeSet<T> union(TreeSet<T> other) => TreeSet<T>.fromList(
      _mergeSortedUnique(toSortedArray(), other.toSortedArray()));

  /// Values present in both sets, in sorted order.
  TreeSet<T> intersection(TreeSet<T> other) => TreeSet<T>.fromList(
      _intersectionSorted(toSortedArray(), other.toSortedArray()));

  /// Values present here but not in [other], in sorted order.
  TreeSet<T> difference(TreeSet<T> other) => TreeSet<T>.fromList(
      _differenceSorted(toSortedArray(), other.toSortedArray()));

  /// Values present in exactly one set, in sorted order.
  TreeSet<T> symmetricDifference(TreeSet<T> other) => TreeSet<T>.fromList(
      _symmetricDifferenceSorted(toSortedArray(), other.toSortedArray()));

  /// True when every value here is also in [other].
  bool isSubset(TreeSet<T> other) =>
      _isSubsetSorted(toSortedArray(), other.toSortedArray());

  /// True when every value in [other] is also here.
  bool isSuperset(TreeSet<T> other) => other.isSubset(this);

  /// True when no value appears in both sets.
  bool isDisjoint(TreeSet<T> other) =>
      _isDisjointSorted(toSortedArray(), other.toSortedArray());

  /// True when both sets hold exactly the same values.
  bool equals(TreeSet<T> other) {
    final a = toSortedArray();
    final b = other.toSortedArray();
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i].compareTo(b[i]) != 0) return false;
    }
    return true;
  }

  @override
  String toString() => 'TreeSet(${toSortedArray()})';
}

/// First index where [sorted] stops being strictly below [bound] —
/// or below-or-equal when [orEqual] — i.e. a partition point.
int _partitionPoint<T extends Comparable<dynamic>>(
    List<T> sorted, T bound, bool orEqual) {
  var lo = 0;
  var hi = sorted.length;
  while (lo < hi) {
    final mid = (lo + hi) ~/ 2;
    final cmp = sorted[mid].compareTo(bound);
    if (cmp < 0 || (orEqual && cmp == 0)) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  return lo;
}

List<T> _mergeSortedUnique<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  final result = <T>[];
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) {
      result.add(left[i++]);
    } else if (cmp > 0) {
      result.add(right[j++]);
    } else {
      result.add(left[i++]);
      j++;
    }
  }
  result.addAll(left.skip(i));
  result.addAll(right.skip(j));
  return result;
}

List<T> _intersectionSorted<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  final result = <T>[];
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) {
      i++;
    } else if (cmp > 0) {
      j++;
    } else {
      result.add(left[i++]);
      j++;
    }
  }
  return result;
}

List<T> _differenceSorted<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  final result = <T>[];
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) {
      result.add(left[i++]);
    } else if (cmp > 0) {
      j++;
    } else {
      i++;
      j++;
    }
  }
  result.addAll(left.skip(i));
  return result;
}

List<T> _symmetricDifferenceSorted<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  final result = <T>[];
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) {
      result.add(left[i++]);
    } else if (cmp > 0) {
      result.add(right[j++]);
    } else {
      i++;
      j++;
    }
  }
  result.addAll(left.skip(i));
  result.addAll(right.skip(j));
  return result;
}

bool _isSubsetSorted<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) return false;
    if (cmp > 0) {
      j++;
    } else {
      i++;
      j++;
    }
  }
  return i == left.length;
}

bool _isDisjointSorted<T extends Comparable<dynamic>>(
    List<T> left, List<T> right) {
  var i = 0;
  var j = 0;
  while (i < left.length && j < right.length) {
    final cmp = left[i].compareTo(right[j]);
    if (cmp < 0) {
      i++;
    } else if (cmp > 0) {
      j++;
    } else {
      return false;
    }
  }
  return true;
}
