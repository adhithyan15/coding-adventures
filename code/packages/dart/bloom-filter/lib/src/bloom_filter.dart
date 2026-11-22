import 'dart:convert';
import 'dart:math' as math;

import 'package:coding_adventures_hash_functions/coding_adventures_hash_functions.dart';

import 'constants.dart';
import 'errors.dart';

int _fmix32(int input) {
  var value = input & mask32;
  value = (value ^ (value >> fmix32Shift1)) & mask32;
  value = (value * fmix32Mul1) & mask32;
  value = (value ^ (value >> fmix32Shift2)) & mask32;
  value = (value * fmix32Mul2) & mask32;
  value = (value ^ (value >> fmix32Shift1)) & mask32;
  return value;
}

void _validateString(String text) {
  final units = text.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final cu = units[i];
    if (cu >= highSurrogateMin && cu <= highSurrogateMax) {
      if (i + 1 >= units.length) {
        throw BloomFilterException.invalidInput();
      }
      final next = units[++i];
      if (next < lowSurrogateMin || next > lowSurrogateMax) {
        throw BloomFilterException.invalidInput();
      }
    } else if (cu >= lowSurrogateMin && cu <= lowSurrogateMax) {
      throw BloomFilterException.invalidInput();
    }
  }
}

/// A persistent Bloom filter with deterministic DT17 double hashing.
///
/// Insertions return new values and never modify the receiver. Inputs are exact
/// bytes or exact UTF-8 strings.
final class BloomFilter {
  /// Total number of bit slots in the filter.
  final int bitCount;

  /// Number of hash functions evaluated per element.
  final int hashCount;

  /// Expected capacity configured at construction (or 0 for explicit sizing).
  final int expectedItems;

  /// Number of insertion operations performed.
  final int itemsAdded;

  final List<int> _bits;
  final int _bitsSet;

  /// Creates a Bloom filter optimal for [expectedItems] and [falsePositiveRate].
  factory BloomFilter({
    int expectedItems = defaultExpectedItems,
    double falsePositiveRate = defaultFalsePositiveRate,
  }) {
    final bitCount = optimalBitCount(
      expectedItems: expectedItems,
      falsePositiveRate: falsePositiveRate,
    );
    final hashCount = optimalHashCount(
      bitCount: bitCount,
      expectedItems: expectedItems,
    );
    final numBytes = (bitCount + bitsPerByte - 1) ~/ bitsPerByte;
    return BloomFilter._(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: expectedItems,
      itemsAdded: 0,
      bits: List<int>.filled(numBytes, 0, growable: false),
      bitsSet: 0,
    );
  }

  /// Creates a Bloom filter with explicit [bitCount] and [hashCount].
  factory BloomFilter.withBitAndHashCount({
    required int bitCount,
    required int hashCount,
  }) {
    _validateBitCount(bitCount);
    _validateHashCount(hashCount);
    final numBytes = (bitCount + bitsPerByte - 1) ~/ bitsPerByte;
    return BloomFilter._(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: 0,
      itemsAdded: 0,
      bits: List<int>.filled(numBytes, 0, growable: false),
      bitsSet: 0,
    );
  }

  const BloomFilter._({
    required this.bitCount,
    required this.hashCount,
    required this.expectedItems,
    required this.itemsAdded,
    required List<int> bits,
    required int bitsSet,
  })  : _bits = bits,
        _bitsSet = bitsSet;

  List<int> _hashIndices(List<int> bytes) {
    final first = BigInt.from(_fmix32(fnv1a32(bytes)));
    final djbVal = (djb2(bytes) & BigInt.from(mask32)).toInt();
    final second = BigInt.from(_fmix32(djbVal) | 1);
    final modulus = BigInt.from(bitCount);
    final indices = <int>[];
    for (var i = 0; i < hashCount; i++) {
      final idx = ((first + BigInt.from(i) * second) % modulus).toInt();
      indices.add(idx);
    }
    return indices;
  }

  /// Return a new filter that has observed these exact [bytes].
  BloomFilter addingBytes(List<int> bytes) {
    for (final byte in bytes) {
      if (byte < 0 || byte > 255) {
        throw BloomFilterException.invalidInput();
      }
    }
    final indices = _hashIndices(bytes);
    var updatedBits = _bits;
    var updatedBitsSet = _bitsSet;
    var changed = false;

    for (final index in indices) {
      final byteIndex = index ~/ bitsPerByte;
      final bitMask = 1 << (index % bitsPerByte);
      if ((updatedBits[byteIndex] & bitMask) == 0) {
        if (!changed) {
          updatedBits = List<int>.from(_bits, growable: false);
          changed = true;
        }
        updatedBits[byteIndex] |= bitMask;
        updatedBitsSet++;
      }
    }

    final newItemsAdded =
        itemsAdded == maxSafeInt ? maxSafeInt : itemsAdded + 1;
    return BloomFilter._(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: expectedItems,
      itemsAdded: newItemsAdded,
      bits: changed ? List<int>.unmodifiable(updatedBits) : _bits,
      bitsSet: updatedBitsSet,
    );
  }

  /// Return a new filter that has observed the string's exact UTF-8 bytes.
  BloomFilter addingString(String text) {
    _validateString(text);
    return addingBytes(utf8.encode(text));
  }

  /// Return a new filter that has observed [value] (either String or List<int>).
  BloomFilter adding(Object value) {
    if (value is String) {
      return addingString(value);
    } else if (value is List<int>) {
      return addingBytes(value);
    } else {
      throw BloomFilterException.invalidInput();
    }
  }

  /// Tests membership of exact [bytes].
  bool containsBytes(List<int> bytes) {
    for (final byte in bytes) {
      if (byte < 0 || byte > 255) {
        throw BloomFilterException.invalidInput();
      }
    }
    for (final index in _hashIndices(bytes)) {
      final byteIndex = index ~/ bitsPerByte;
      final bitMask = 1 << (index % bitsPerByte);
      if ((_bits[byteIndex] & bitMask) == 0) {
        return false;
      }
    }
    return true;
  }

  /// Tests membership of string's UTF-8 bytes.
  bool containsString(String text) {
    _validateString(text);
    return containsBytes(utf8.encode(text));
  }

  /// Tests membership of [value] (either String or List<int>).
  bool contains(Object value) {
    if (value is String) {
      return containsString(value);
    } else if (value is List<int>) {
      return containsBytes(value);
    } else {
      throw BloomFilterException.invalidInput();
    }
  }

  /// Total number of 1-bits currently set.
  int get bitsSet => _bitsSet;

  /// Proportion of bit slots currently set to 1.
  double get fillRatio => _bitsSet.toDouble() / bitCount.toDouble();

  /// Estimated false positive probability based on current fill ratio.
  double get estimatedFalsePositiveRate => _bitsSet == 0
      ? 0.0
      : math.pow(fillRatio, hashCount.toDouble()).toDouble();

  /// Whether the number of inserted elements exceeds [expectedItems].
  bool get isOverCapacity => expectedItems > 0 && itemsAdded > expectedItems;

  /// Memory size in bytes of the bit array.
  int get sizeBytes => _bits.length;

  /// Immutable snapshot of internal bytes.
  List<int> get bits => List<int>.unmodifiable(_bits);

  /// Computes the optimal bit count for given item count and error rate.
  static int optimalBitCount({
    required int expectedItems,
    required double falsePositiveRate,
  }) {
    _validateExpectedItems(expectedItems);
    _validateFalsePositiveRate(falsePositiveRate);
    final logTwo = math.log(2.0);
    final requested = (-expectedItems.toDouble() *
            math.log(falsePositiveRate) /
            (logTwo * logTwo))
        .ceil();
    if (!requested.isFinite || requested > maximumBitCount) {
      final bounded = requested.isFinite && requested < maxSafeInt
          ? requested.toInt()
          : maxSafeInt;
      throw BloomFilterException.allocationLimitExceeded(
        bounded,
        maximumBitCount,
      );
    }
    final bitCount = requested.toInt();
    _validateBitCount(bitCount);
    return bitCount;
  }

  /// Computes the optimal number of hash functions.
  static int optimalHashCount({
    required int bitCount,
    required int expectedItems,
  }) {
    _validateBitCount(bitCount);
    _validateExpectedItems(expectedItems);
    final requested = math.max(
      1,
      (bitCount.toDouble() / expectedItems.toDouble() * math.log(2.0)).round(),
    );
    _validateHashCount(requested);
    return requested;
  }

  /// Computes maximum capacity for a given memory footprint and error rate.
  static int capacity({
    required int memoryBytes,
    required double falsePositiveRate,
  }) {
    if (memoryBytes <= 0 || memoryBytes > maximumBitCount ~/ bitsPerByte) {
      throw BloomFilterException.invalidMemoryBytes(memoryBytes);
    }
    _validateFalsePositiveRate(falsePositiveRate);
    final bitCount = (memoryBytes * bitsPerByte).toDouble();
    final logTwo = math.log(2.0);
    final cap =
        (-bitCount * logTwo * logTwo / math.log(falsePositiveRate)).floor();
    if (!cap.isFinite || cap >= maxSafeInt) {
      throw BloomFilterException.capacityOverflow();
    }
    return cap.toInt();
  }

  static void _validateExpectedItems(int expectedItems) {
    if (expectedItems <= 0) {
      throw BloomFilterException.invalidExpectedItems(expectedItems);
    }
  }

  static void _validateFalsePositiveRate(double rate) {
    if (!rate.isFinite || rate <= 0.0 || rate >= 1.0) {
      throw BloomFilterException.invalidFalsePositiveRate(rate);
    }
  }

  static void _validateBitCount(int bitCount) {
    if (bitCount <= 0) {
      throw BloomFilterException.invalidBitCount(bitCount);
    }
    if (bitCount > maximumBitCount) {
      throw BloomFilterException.allocationLimitExceeded(
        bitCount,
        maximumBitCount,
      );
    }
  }

  static void _validateHashCount(int hashCount) {
    if (hashCount <= 0 || hashCount > maximumHashCount) {
      throw BloomFilterException.invalidHashCount(hashCount);
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! BloomFilter ||
        bitCount != other.bitCount ||
        hashCount != other.hashCount ||
        expectedItems != other.expectedItems ||
        itemsAdded != other.itemsAdded ||
        _bitsSet != other._bitsSet) {
      return false;
    }
    for (var i = 0; i < _bits.length; i++) {
      if (_bits[i] != other._bits[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode {
    var hash =
        Object.hash(bitCount, hashCount, expectedItems, itemsAdded, _bitsSet);
    for (final b in _bits) {
      hash = (hash * 31 + b) & maxSafeInt;
    }
    return hash;
  }
}
