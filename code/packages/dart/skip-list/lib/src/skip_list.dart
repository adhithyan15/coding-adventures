/// Sorted skip-list collection: the Dart lane of DT20.
///
/// A skip list keeps elements sorted with O(log n) expected search, insert,
/// and delete — plus range queries and rank access that hash maps cannot do.
/// This lane stores entries in a balanced ordered map (like the reference
/// lanes), so all observable behavior is deterministic: sorted iteration,
/// point lookups, deletions, rank queries, and range queries. The
/// `maxLevel`/`probability` parameters and the `currentMax` height estimate
/// preserve the skip-list tuning surface.
///
/// ```dart
/// import 'package:coding_adventures_skip_list/coding_adventures_skip_list.dart';
///
/// void main() {
///   final scores = SkipList<String, int>();
///   scores.insert('ada', 1815);
///   scores.insert('grace', 1906);
///   print(scores.search('ada')); // 1815
///   print(scores.toList()); // [ada, grace]
///   print(scores.rangeQuery('a', 'm', true)); // [(ada, 1815), (grace, 1906)]
/// }
/// ```
library skip_list;

import 'dart:collection';
import 'dart:math' as math;

/// An ordered key/value collection with skip-list semantics.
///
/// Keys sort via [Comparable]; values are unrestricted. All snapshots
/// ([toList], [entries]) copy, so later mutations never affect them.
class SkipList<K extends Comparable<dynamic>, V> {
  final SplayTreeMap<K, V> _entries = SplayTreeMap<K, V>();
  final int _maxLevel;
  final double _probability;

  /// Creates an empty list with default tuning (32 levels, p = 0.5).
  SkipList() : this._tuned(32, 0.5);

  /// Creates an empty list with explicit tuning.
  ///
  /// `maxLevel` clamps to at least 1; a non-finite `probability` outside
  /// (0, 1) falls back to 0.5 — matching the reference lanes.
  factory SkipList.withParams(int maxLevel, double probability) =>
      SkipList._tuned(
        math.max(1, maxLevel),
        probability.isFinite && probability > 0.0 && probability < 1.0
            ? probability
            : 0.5,
      );

  /// Alias of [SkipList.withParams] kept for cross-lane naming parity.
  factory SkipList.newWithParams(int maxLevel, double probability) =>
      SkipList.withParams(maxLevel, probability);

  SkipList._tuned(this._maxLevel, this._probability);

  /// Inserts (or replaces) [key] with [value].
  void insert(K key, V value) {
    _entries[key] = value;
  }

  /// Removes [key], returning true if it was present.
  ///
  /// Presence is checked before removal because a stored null value would
  /// otherwise be indistinguishable from a missing key.
  bool delete(K key) {
    if (!_entries.containsKey(key)) return false;
    _entries.remove(key);
    return true;
  }

  /// Looks up [key], or null when absent.
  V? search(K key) => _entries[key];

  /// Returns true if [key] is present.
  bool contains(K key) => _entries.containsKey(key);

  /// Alias of [contains] kept for cross-lane naming parity.
  bool containsKey(K key) => contains(key);

  /// 0-based position of [key] in sorted order, or null when absent.
  int? rank(K key) {
    var index = 0;
    for (final current in _entries.keys) {
      if (current.compareTo(key) == 0) return index;
      index++;
    }
    return null;
  }

  /// Key at 0-based [rank] in sorted order, or null when out of range.
  K? byRank(int rank) {
    if (rank < 0) return null;
    var index = 0;
    for (final key in _entries.keys) {
      if (index == rank) return key;
      index++;
    }
    return null;
  }

  /// Entries with `lo <= key <= hi` (or strict, when not [inclusive]).
  ///
  /// Returns `[]` when `lo > hi`. Bounds compare with the key ordering.
  List<(K, V)> rangeQuery(K lo, K hi, bool inclusive) =>
      range(lo, hi, inclusive);

  /// Alias of [rangeQuery] kept for cross-lane naming parity.
  List<(K, V)> range(K lo, K hi, bool inclusive) {
    if (lo.compareTo(hi) > 0) return [];
    final out = <(K, V)>[];
    for (final entry in _entries.entries) {
      final aboveLo = inclusive
          ? entry.key.compareTo(lo) >= 0
          : entry.key.compareTo(lo) > 0;
      final belowHi = inclusive
          ? entry.key.compareTo(hi) <= 0
          : entry.key.compareTo(hi) < 0;
      if (aboveLo && belowHi) out.add((entry.key, entry.value));
    }
    return out;
  }

  /// All keys in sorted order.
  List<K> toList() => _entries.keys.toList();

  /// All entries in sorted key order.
  List<(K, V)> entries() => entriesList();

  /// Alias of [entries] kept for cross-lane naming parity.
  List<(K, V)> entriesList() =>
      [for (final e in _entries.entries) (e.key, e.value)];

  /// Smallest key, or null when empty.
  K? min() => _entries.isEmpty ? null : _entries.firstKey();

  /// Largest key, or null when empty.
  K? max() => _entries.isEmpty ? null : _entries.lastKey();

  /// Number of entries.
  int get length => _entries.length;

  /// Alias of [length] kept for cross-lane naming parity.
  int size() => length;

  /// True when no entries are stored.
  bool get isEmpty => _entries.isEmpty;

  /// True when at least one entry is stored.
  bool get isNotEmpty => _entries.isNotEmpty;

  /// Configured maximum level.
  int maxLevel() => _maxLevel;

  /// Configured level-up probability.
  double probability() => _probability;

  /// Estimated current height: `ceil(log(len) / log(1/p))` clamped to
  /// `[1, maxLevel]`; 1 when empty. Matches the reference formula exactly.
  int currentMax() {
    if (_entries.isEmpty) return 1;
    final levels =
        (math.log(_entries.length) / math.log(1.0 / _probability)).ceil();
    return levels.clamp(1, _maxLevel);
  }

  /// Keys in sorted order.
  Iterable<K> get keys => _entries.keys;

  @override
  String toString() => 'SkipList(${toList()})';
}
