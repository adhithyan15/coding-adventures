import 'package:coding_adventures_hash_map/coding_adventures_hash_map.dart';
import 'package:test/test.dart';

import 'hash_map_shared_fixture_test.dart' as fixture;

void main() {
  test(
    'public maximum-capacity chaining accepts its boundary and rejects growth',
    () {
      const maximum = HashMapLimits.maximumCapacity;
      final before = HashMap<int>.fromEntries(
        _entries(maximum),
        capacity: maximum,
        strategy: CollisionStrategy.separateChaining,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      );
      expect(before.count, maximum);
      expect(before.capacity, maximum);
      expect(before.has(_key(maximum - 1)), isTrue);
      expect(before.valueForKey(_key(maximum - 1)), maximum - 1);

      final distinct = _key(maximum);
      final growthError = _captureError(
        () => before.setting(-1, forKey: distinct),
      );
      expect(growthError, isA<CapacityLimitExceeded>());
      expect(before.count, maximum);
      expect(before.capacity, maximum);
      expect(before.has(distinct), isFalse);

      final replacement = before.setting(-7, forKey: _key(maximum - 1));
      expect(replacement.count, maximum);
      expect(replacement.capacity, maximum);
      expect(replacement.valueForKey(_key(maximum - 1)), -7);
      expect(before.valueForKey(_key(maximum - 1)), maximum - 1);

      final afterRemove = before.removingValueForKey(_key(maximum - 1));
      expect(afterRemove.count, maximum - 1);
      expect(afterRemove.capacity, maximum);
      final reinserted = afterRemove.setting(100, forKey: distinct);
      expect(reinserted.count, maximum);
      expect(reinserted.capacity, maximum);
      expect(reinserted.valueForKey(distinct), 100);
      expect(afterRemove.has(distinct), isFalse);
      expect(before.has(_key(maximum - 1)), isTrue);
      print(
        'DT18 public chaining boundary: capacity=$maximum, entries=$maximum, '
        'distinct-at-limit rejected; replace and delete/reinsert preserved '
        'capacity and earlier versions.',
      );
    },
    timeout: const Timeout(Duration(minutes: 20)),
  );

  test(
    'public maximum-capacity probing pins the 75 percent transition',
    () {
      const maximum = HashMapLimits.maximumCapacity;
      const threshold = maximum * 3 ~/ 4;
      final before = HashMap<int>.fromEntries(
        _entries(threshold),
        capacity: maximum,
        strategy: CollisionStrategy.linearProbing,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      );
      expect(before.count, threshold);
      expect(before.capacity, maximum);
      expect(before.has(_key(threshold - 1)), isTrue);
      expect(before.valueForKey(_key(threshold - 1)), threshold - 1);

      final distinct = _key(threshold);
      final growthError = _captureError(
        () => before.setting(-1, forKey: distinct),
      );
      expect(growthError, isA<CapacityLimitExceeded>());
      expect(before.count, threshold);
      expect(before.capacity, maximum);
      expect(before.has(distinct), isFalse);

      final replacement = before.setting(-9, forKey: _key(threshold - 1));
      expect(replacement.count, threshold);
      expect(replacement.capacity, maximum);
      expect(replacement.valueForKey(_key(threshold - 1)), -9);
      expect(before.valueForKey(_key(threshold - 1)), threshold - 1);

      final afterRemove = before.removingValueForKey(_key(threshold - 1));
      expect(afterRemove.count, threshold - 1);
      expect(afterRemove.capacity, maximum);
      final reinserted = afterRemove.setting(101, forKey: distinct);
      expect(reinserted.count, threshold);
      expect(reinserted.capacity, maximum);
      expect(reinserted.valueForKey(distinct), 101);
      expect(afterRemove.has(distinct), isFalse);
      expect(before.has(_key(threshold - 1)), isTrue);
      print(
        'DT18 public probing boundary: capacity=$maximum, entries=$threshold '
        '(75%); crossing-at-limit rejected, replacement and delete/reinsert '
        'preserved the exact threshold and earlier versions.',
      );
    },
    timeout: const Timeout(Duration(minutes: 20)),
  );

  test('public key payload accepts exactly 1 MiB and rejects one byte more',
      () {
    const maximum = HashMapLimits.maximumKeyByteCount;
    final maximumKey = HashMapKey.bytes(List<int>.filled(maximum, 0x5a));
    expect(maximumKey.encodedBytes, hasLength(maximum + 9));
    final map = HashMap<int>(
      capacity: 3,
      strategy: CollisionStrategy.separateChaining,
      hashAlgorithm: const HashAlgorithm.fnv1a32(),
    ).setting(1, forKey: maximumKey);
    expect(map.count, 1);
    expect(map.valueForKey(maximumKey), 1);

    final tooLong = HashMapKey.bytes(List<int>.filled(maximum + 1, 0x5a));
    final error = _captureError(() => map.setting(2, forKey: tooLong));
    expect(error, isA<KeyTooLong>());
    final typed = error as KeyTooLong;
    expect(typed.actual, maximum + 1);
    expect(typed.maximum, maximum);
    expect(map.has(tooLong), isFalse);
    expect(map.valueForKey(tooLong), isNull);
    final afterMissingRemoval = map.removingValueForKey(tooLong);
    expect(afterMissingRemoval.count, map.count);
    expect(afterMissingRemoval.capacity, map.capacity);
    expect(afterMissingRemoval.valueForKey(maximumKey), 1);
    expect(map.count, 1);
    expect(map.valueForKey(maximumKey), 1);
    print(
      'DT18 public key boundary: accepted payload=$maximum bytes; rejected '
      'payload=${maximum + 1} bytes with typed error and unchanged map.',
    );
  }, timeout: const Timeout(Duration(minutes: 10)));
}

Iterable<MapEntry<HashMapKey, int>> _entries(int count) sync* {
  for (var value = 0; value < count; value++) {
    yield MapEntry<HashMapKey, int>(_key(value), value);
  }
}

HashMapKey _key(int value) => HashMapKey.bytes([
      value >> 16 & 0xff,
      value >> 8 & 0xff,
      value & 0xff,
    ]);

Object _captureError(void Function() operation) {
  try {
    operation();
  } catch (error) {
    return error;
  }
  throw StateError('Expected public operation to throw.');
}
