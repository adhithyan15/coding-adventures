part of coding_adventures_hash_map;

sealed class HashMapError implements Exception {
  const HashMapError();
}

final class InvalidCapacity extends HashMapError {
  const InvalidCapacity(this.capacity);
  final int capacity;
}

final class CapacityLimitExceeded extends HashMapError {
  const CapacityLimitExceeded();
}

final class InvalidSipHashKeyLength extends HashMapError {
  const InvalidSipHashKeyLength(this.actualLength);
  final int actualLength;
}

final class InvalidMurmurSeed extends HashMapError {
  const InvalidMurmurSeed(this.seed);
  final int seed;
}

final class InvalidByte extends HashMapError {
  const InvalidByte(this.index, this.value);
  final int index;
  final int value;
}

final class MalformedUtf16 extends HashMapError {
  const MalformedUtf16(this.codeUnitOffset);
  final int codeUnitOffset;
}

final class KeyTooLong extends HashMapError {
  const KeyTooLong(this.actual, this.maximum);
  final int actual;
  final int maximum;
}
