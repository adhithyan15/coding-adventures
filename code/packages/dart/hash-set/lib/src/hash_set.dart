import 'package:coding_adventures_hash_map/coding_adventures_hash_map.dart';

import 'constants.dart';

/// A persistent DT19 hash set backed by the DT18 `HashMap<bool>` package.
///
/// A set needs only the key half of a map. The private `bool` value is a
/// sentinel: membership is always decided with `HashMap.has`, never by reading
/// that value. Every operation returns a new set, so older values remain valid.
final class HashSet {
  final HashMap<bool> _storage;

  const HashSet._(this._storage);

  /// Create an empty set with a finite, explicit hash-table profile.
  factory HashSet({
    int capacity = defaultCapacity,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  }) {
    return HashSet._(HashMap<bool>(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm,
    ));
  }

  /// Build a set from keys, discarding duplicates.
  factory HashSet.fromElements(
    Iterable<HashMapKey> elements, {
    int capacity = defaultCapacity,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  }) {
    var result = HashSet(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm,
    );
    for (final element in elements) {
      result = result.adding(element);
    }
    return result;
  }

  /// The current table profile. Capacity may grow as elements are inserted.
  HashMapConfiguration get configuration => _storage.configuration;

  /// Number of distinct elements.
  int get count => _storage.count;

  /// Current capacity of the underlying hash table.
  int get capacity => _storage.capacity;

  /// The collision strategy used by the underlying hash table.
  CollisionStrategy get strategy => _storage.strategy;

  /// The hash algorithm used by the underlying hash table.
  HashAlgorithm get hashAlgorithm => _storage.hashAlgorithm;

  /// True if the set contains zero elements.
  bool get isEmpty => count == 0;

  /// True if the set contains at least one element.
  bool get isNotEmpty => count > 0;

  /// Elements in the underlying map's deterministic physical storage order.
  ///
  /// This is not insertion or sorted order. It is stable only for an identical
  /// strategy, hash profile, initial capacity, and operation history.
  List<HashMapKey> get elements => _storage.keys;

  /// Tests membership of [element].
  bool contains(HashMapKey element) => _storage.has(element);

  /// Return a new set containing [element].
  HashSet adding(HashMapKey element) =>
      HashSet._(_storage.setting(sentinelValue, forKey: element));

  /// Return a new set without [element]; a missing element is a no-op.
  HashSet removing(HashMapKey element) =>
      HashSet._(_storage.removingValueForKey(element));

  /// Python-style spelling for non-failing removal.
  HashSet discarding(HashMapKey element) => removing(element);

  // MARK: - Set algebra

  /// Return A ∪ B while retaining the receiver's table profile.
  HashSet union(HashSet other) {
    var result = this;
    for (final element in other.elements) {
      result = result.adding(element);
    }
    return result;
  }

  /// Return A ∩ B. Iterate the smaller input but retain the receiver profile.
  HashSet intersection(HashSet other) {
    final smaller = count <= other.count ? this : other;
    final larger = count <= other.count ? other : this;
    var result = _emptyWithReceiverProfile();
    for (final element in smaller.elements) {
      if (larger.contains(element)) {
        result = result.adding(element);
      }
    }
    return result;
  }

  /// Return A − B retaining the receiver profile.
  HashSet difference(HashSet other) {
    var result = _emptyWithReceiverProfile();
    for (final element in elements) {
      if (!other.contains(element)) {
        result = result.adding(element);
      }
    }
    return result;
  }

  /// Return keys in exactly one input, A △ B, retaining the receiver profile.
  HashSet symmetricDifference(HashSet other) {
    var result = _emptyWithReceiverProfile();
    for (final element in elements) {
      if (!other.contains(element)) {
        result = result.adding(element);
      }
    }
    for (final element in other.elements) {
      if (!contains(element)) {
        result = result.adding(element);
      }
    }
    return result;
  }

  // MARK: - Relations

  /// True if every element in this set is also contained in [other].
  bool isSubsetOf(HashSet other) {
    if (count > other.count) return false;
    for (final element in elements) {
      if (!other.contains(element)) return false;
    }
    return true;
  }

  /// Alias for [isSubsetOf].
  bool isSubset(HashSet other) => isSubsetOf(other);

  /// True if this set contains every element in [other].
  bool isSupersetOf(HashSet other) => other.isSubsetOf(this);

  /// Alias for [isSupersetOf].
  bool isSuperset(HashSet other) => isSupersetOf(other);

  /// True if this set and [other] share no common elements.
  bool isDisjointWith(HashSet other) {
    final smaller = count <= other.count ? this : other;
    final larger = count <= other.count ? other : this;
    for (final element in smaller.elements) {
      if (larger.contains(element)) return false;
    }
    return true;
  }

  /// Alias for [isDisjointWith].
  bool isDisjoint(HashSet other) => isDisjointWith(other);

  /// True if both sets contain the same elements regardless of order.
  bool equals(HashSet other) => count == other.count && isSubsetOf(other);

  @override
  bool operator ==(Object other) => other is HashSet && equals(other);

  @override
  int get hashCode {
    var h = 0;
    for (final elem in elements) {
      h ^= elem.hashCode;
    }
    return Object.hash(count, h);
  }

  HashSet _emptyWithReceiverProfile() {
    final profile = configuration;
    return HashSet(
      capacity: profile.capacity,
      strategy: profile.strategy,
      hashAlgorithm: profile.hashAlgorithm,
    );
  }
}
