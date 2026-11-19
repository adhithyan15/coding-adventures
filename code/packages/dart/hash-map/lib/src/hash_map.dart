part of coding_adventures_hash_map;

enum CollisionStrategy { separateChaining, linearProbing }

final class HashMapConfiguration {
  const HashMapConfiguration._({
    required this.capacity,
    required this.strategy,
    required this.hashAlgorithm,
  });

  final int capacity;
  final CollisionStrategy strategy;
  final HashAlgorithm hashAlgorithm;

  @override
  bool operator ==(Object other) =>
      other is HashMapConfiguration &&
      other.capacity == capacity &&
      other.strategy == strategy &&
      other.hashAlgorithm == hashAlgorithm;

  @override
  int get hashCode => Object.hash(capacity, strategy, hashAlgorithm);
}

abstract final class HashMapLimits {
  static const int maximumCapacity = 1048576;
  static const int maximumKeyByteCount = 1048576;
  static const List<int> deterministicSipHashKey = [
    0x63,
    0x6f,
    0x64,
    0x65,
    0x78,
    0x2d,
    0x64,
    0x74,
    0x31,
    0x38,
    0x2d,
    0x6b,
    0x65,
    0x79,
    0x21,
    0x21,
  ];
}

/// An immutable DT18 table. Updates publish new structural state.
final class HashMap<V> {
  HashMap._(this._table);

  factory HashMap({
    int capacity = 16,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  }) {
    final algorithm = hashAlgorithm ??
        HashAlgorithm.sipHash24(key: HashMapLimits.deterministicSipHashKey);
    // Capacity is checked before deferred algorithm parameters by contract.
    _validateMapConfiguration(capacity, algorithm);
    return HashMap._(_Table<V>.empty(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: algorithm,
    ));
  }

  factory HashMap.fromEntries(
    Iterable<MapEntry<HashMapKey, V>> entries, {
    int capacity = 16,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  }) {
    final algorithm = hashAlgorithm ??
        HashAlgorithm.sipHash24(key: HashMapLimits.deterministicSipHashKey);
    _validateMapConfiguration(capacity, algorithm);
    // The private builder consumes entries sequentially and publishes only after
    // every entry succeeds. This keeps million-entry setup linear and atomic.
    final builder = _TableBuilder<V>.empty(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: algorithm,
    );
    for (final entry in entries) {
      builder.set(entry.key, entry.value);
    }
    return HashMap._(builder.freeze());
  }

  final _Table<V> _table;

  int get count => _table.count;
  int get capacity => _table.capacity;
  double get loadFactor => count / capacity;
  CollisionStrategy get strategy => _table.strategy;
  HashAlgorithm get hashAlgorithm => _table.hashAlgorithm;

  HashMapConfiguration get configuration => HashMapConfiguration._(
        capacity: capacity,
        strategy: strategy,
        hashAlgorithm: hashAlgorithm,
      );

  List<MapEntry<HashMapKey, V>> get entries => _table.entries();
  List<HashMapKey> get keys => [for (final entry in entries) entry.key];
  List<V> get values => [for (final entry in entries) entry.value];

  /// Copies table structure and returns a new version with [forKey] set.
  ///
  /// Entry values are shallow references. A replacement keeps its physical
  /// position and never triggers growth.
  HashMap<V> setting(V value, {required HashMapKey forKey}) {
    final length = forKey._payloadByteCount;
    if (length > HashMapLimits.maximumKeyByteCount) {
      throw KeyTooLong(length, HashMapLimits.maximumKeyByteCount);
    }
    final builder = _TableBuilder<V>.fromTable(_table);
    builder.set(forKey, value);
    return HashMap._(builder.freeze());
  }

  V? valueForKey(HashMapKey key) {
    if (key._payloadByteCount > HashMapLimits.maximumKeyByteCount) return null;
    return _table.find(key)?.value;
  }

  bool has(HashMapKey key) {
    if (key._payloadByteCount > HashMapLimits.maximumKeyByteCount) return false;
    return _table.find(key) != null;
  }

  HashMap<V> removingValueForKey(HashMapKey key) {
    if (key._payloadByteCount > HashMapLimits.maximumKeyByteCount) return this;
    if (_table.find(key) == null) return this;
    final builder = _TableBuilder<V>.fromTable(_table);
    if (!builder.remove(key)) return this;
    return HashMap._(builder.freeze());
  }

  /// Applies right-side entries in physical order using this map's settings.
  ///
  /// The builder remains private, so failure cannot publish a partial result.
  HashMap<V> merging(HashMap<V> other) {
    final builder = _TableBuilder<V>.fromTable(_table);
    for (final entry in other.entries) {
      builder.set(entry.key, entry.value);
    }
    return HashMap._(builder.freeze());
  }
}

void _validateMapConfiguration(int capacity, HashAlgorithm algorithm) {
  if (capacity <= 0) throw InvalidCapacity(capacity);
  if (capacity > HashMapLimits.maximumCapacity) {
    throw const CapacityLimitExceeded();
  }
  algorithm._validateConfiguration();
}

final class _Entry<V> {
  const _Entry(this.key, this.value);
  final HashMapKey key;
  final V value;
}

enum _SlotState { empty, tombstone, occupied }

final class _Slot<V> {
  const _Slot.empty()
      : state = _SlotState.empty,
        entry = null;
  const _Slot.tombstone()
      : state = _SlotState.tombstone,
        entry = null;
  const _Slot.occupied(_Entry<V> value)
      : state = _SlotState.occupied,
        entry = value;

  final _SlotState state;
  final _Entry<V>? entry;
}

final class _Table<V> {
  const _Table({
    required this.count,
    required this.capacity,
    required this.strategy,
    required this.hashAlgorithm,
    required this.buckets,
    required this.slots,
  });

  factory _Table.empty({
    required int capacity,
    required CollisionStrategy strategy,
    required HashAlgorithm hashAlgorithm,
  }) {
    if (strategy == CollisionStrategy.separateChaining) {
      return _Table<V>(
        count: 0,
        capacity: capacity,
        strategy: strategy,
        hashAlgorithm: hashAlgorithm,
        buckets: List<List<_Entry<V>>>.generate(
          capacity,
          (_) => <_Entry<V>>[],
          growable: false,
        ),
        slots: null,
      );
    }
    return _Table<V>(
      count: 0,
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm,
      buckets: null,
      slots: List<_Slot<V>>.filled(capacity, _Slot<V>.empty()),
    );
  }

  final int count;
  final int capacity;
  final CollisionStrategy strategy;
  final HashAlgorithm hashAlgorithm;
  final List<List<_Entry<V>>>? buckets;
  final List<_Slot<V>>? slots;

  // Observable order follows physical storage: ascending bucket/slot, with
  // chaining collisions in their insertion order.
  List<MapEntry<HashMapKey, V>> entries() {
    if (strategy == CollisionStrategy.separateChaining) {
      return [
        for (final bucket in buckets!)
          for (final entry in bucket) MapEntry(entry.key, entry.value),
      ];
    }
    return [
      for (final slot in slots!)
        if (slot.state == _SlotState.occupied)
          MapEntry(slot.entry!.key, slot.entry!.value),
    ];
  }

  // Probing stops at an empty slot or after one bounded full-table traversal.
  // Tombstones preserve the search chain and are skipped.
  _Entry<V>? find(HashMapKey key) {
    if (strategy == CollisionStrategy.separateChaining) {
      final bucket = buckets![_bucketIndex(key, capacity, hashAlgorithm)];
      for (final entry in bucket) {
        if (entry.key == key) return entry;
      }
      return null;
    }
    final slots = this.slots!;
    final start = _bucketIndex(key, capacity, hashAlgorithm);
    for (var offset = 0; offset < capacity; offset++) {
      final slot = slots[(start + offset) % capacity];
      if (slot.state == _SlotState.empty) return null;
      if (slot.state == _SlotState.occupied && slot.entry!.key == key) {
        return slot.entry;
      }
    }
    return null;
  }
}

/// Mutable state used only while a result is being built.
///
/// Existing maps share immutable bucket/slot objects, while each update copies
/// the outer list and replaces changed storage. The builder never escapes.
final class _TableBuilder<V> {
  _TableBuilder({
    required this.count,
    required this.capacity,
    required this.strategy,
    required this.hashAlgorithm,
    required this.buckets,
    required this.slots,
  });

  factory _TableBuilder.empty({
    required int capacity,
    required CollisionStrategy strategy,
    required HashAlgorithm hashAlgorithm,
  }) {
    final chaining = strategy == CollisionStrategy.separateChaining;
    return _TableBuilder<V>(
      count: 0,
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm,
      buckets: chaining
          ? List<List<_Entry<V>>>.generate(
              capacity,
              (_) => <_Entry<V>>[],
              growable: false,
            )
          : null,
      slots:
          chaining ? null : List<_Slot<V>>.filled(capacity, _Slot<V>.empty()),
    );
  }

  factory _TableBuilder.fromTable(_Table<V> table) => _TableBuilder<V>(
        count: table.count,
        capacity: table.capacity,
        strategy: table.strategy,
        hashAlgorithm: table.hashAlgorithm,
        buckets: table.buckets == null ? null : List.of(table.buckets!),
        slots: table.slots == null ? null : List.of(table.slots!),
      );

  int count;
  int capacity;
  final CollisionStrategy strategy;
  final HashAlgorithm hashAlgorithm;
  List<List<_Entry<V>>>? buckets;
  List<_Slot<V>>? slots;

  _Table<V> freeze() => _Table<V>(
        count: count,
        capacity: capacity,
        strategy: strategy,
        hashAlgorithm: hashAlgorithm,
        buckets: buckets,
        slots: slots,
      );

  void set(HashMapKey key, V value) {
    final payloadLength = key._payloadByteCount;
    if (payloadLength > HashMapLimits.maximumKeyByteCount) {
      throw KeyTooLong(payloadLength, HashMapLimits.maximumKeyByteCount);
    }

    if (strategy == CollisionStrategy.separateChaining) {
      final oldIndex = _bucketIndex(key, capacity, hashAlgorithm);
      final oldBucket = buckets![oldIndex];
      final existing = oldBucket.indexWhere((entry) => entry.key == key);
      if (existing >= 0) {
        final replacement = List<_Entry<V>>.of(oldBucket);
        replacement[existing] = _Entry<V>(key, value);
        buckets![oldIndex] = replacement;
        return;
      }
      _ensureCapacityForNewEntry();
      final index = _bucketIndex(key, capacity, hashAlgorithm);
      buckets![index] = [...buckets![index], _Entry<V>(key, value)];
      count++;
      return;
    }

    final existingIndex = _findSlotIndex(key);
    if (existingIndex != null) {
      slots![existingIndex] = _Slot<V>.occupied(_Entry<V>(key, value));
      return;
    }
    _ensureCapacityForNewEntry();
    _insertLinear(_Entry<V>(key, value));
  }

  bool remove(HashMapKey key) {
    if (strategy == CollisionStrategy.separateChaining) {
      final index = _bucketIndex(key, capacity, hashAlgorithm);
      final bucket = buckets![index];
      final entryIndex = bucket.indexWhere((entry) => entry.key == key);
      if (entryIndex < 0) return false;
      final replacement = List<_Entry<V>>.of(bucket)..removeAt(entryIndex);
      buckets![index] = replacement;
      count--;
      return true;
    }
    final index = _findSlotIndex(key);
    if (index == null) return false;
    slots![index] = _Slot<V>.tombstone();
    count--;
    return true;
  }

  int? _findSlotIndex(HashMapKey key) {
    final slots = this.slots!;
    final start = _bucketIndex(key, capacity, hashAlgorithm);
    for (var offset = 0; offset < capacity; offset++) {
      final index = (start + offset) % capacity;
      final slot = slots[index];
      if (slot.state == _SlotState.empty) return null;
      if (slot.state == _SlotState.occupied && slot.entry!.key == key) {
        return index;
      }
    }
    return null;
  }

  // Replacement is detected before this method. The threshold therefore only
  // applies to a distinct key and, for probing, runs before tombstone reuse.
  void _ensureCapacityForNewEntry() {
    final shouldGrow = strategy == CollisionStrategy.separateChaining
        ? count == capacity
        : (count + 1) * 4 > capacity * 3;
    if (!shouldGrow) return;

    final doubled = capacity * 2;
    final newCapacity = doubled < HashMapLimits.maximumCapacity
        ? doubled
        : HashMapLimits.maximumCapacity;
    if (newCapacity <= capacity) throw const CapacityLimitExceeded();
    _rehash(newCapacity);
  }

  // Reinsert the old physical sequence so the documented iteration order is
  // stable across a resize. Deleted probe slots are discarded here.
  void _rehash(int newCapacity) {
    final oldEntries = entriesInPhysicalOrder();
    capacity = newCapacity;
    count = 0;
    if (strategy == CollisionStrategy.separateChaining) {
      buckets = List<List<_Entry<V>>>.generate(
        capacity,
        (_) => <_Entry<V>>[],
        growable: false,
      );
      slots = null;
      for (final entry in oldEntries) {
        final index = _bucketIndex(entry.key, capacity, hashAlgorithm);
        buckets![index] = [...buckets![index], entry];
        count++;
      }
      return;
    }
    buckets = null;
    slots = List<_Slot<V>>.filled(capacity, _Slot<V>.empty());
    for (final entry in oldEntries) {
      _insertLinear(entry);
    }
  }

  List<_Entry<V>> entriesInPhysicalOrder() {
    if (strategy == CollisionStrategy.separateChaining) {
      return [for (final bucket in buckets!) ...bucket];
    }
    return [
      for (final slot in slots!)
        if (slot.state == _SlotState.occupied) slot.entry!,
    ];
  }

  // Remember the earliest tombstone but keep searching for a duplicate.
  // After one complete cycle, reuse that tombstone if the table has no empty
  // slot; a low live count alone does not guarantee an empty slot.
  void _insertLinear(_Entry<V> entry) {
    final slots = this.slots!;
    final start = _bucketIndex(entry.key, capacity, hashAlgorithm);
    int? firstTombstone;
    for (var offset = 0; offset < capacity; offset++) {
      final index = (start + offset) % capacity;
      final slot = slots[index];
      if (slot.state == _SlotState.empty) {
        slots[firstTombstone ?? index] = _Slot<V>.occupied(entry);
        count++;
        return;
      }
      if (slot.state == _SlotState.tombstone && firstTombstone == null) {
        firstTombstone = index;
      }
    }
    if (firstTombstone != null) {
      slots[firstTombstone] = _Slot<V>.occupied(entry);
      count++;
      return;
    }
    throw const CapacityLimitExceeded();
  }
}

// Reduce the unsigned result while it is still a BigInt. Converting the full
// 64-bit value to a JS number before modulo would lose bucket-index bits.
int _bucketIndex(HashMapKey key, int capacity, HashAlgorithm algorithm) =>
    (algorithm._hashValidated(key.encodedBytes) % BigInt.from(capacity))
        .toInt();
