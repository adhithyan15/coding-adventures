import 'dart:collection';

/// Number of addressable bits in each internal word.
const int bitsPerWord = 64;

final BigInt _wordMask = (BigInt.one << bitsPerWord) - BigInt.one;

/// Error raised when a Bitset constructor receives invalid data.
class BitsetError implements Exception {
  const BitsetError(this.message);

  final String message;

  @override
  String toString() => 'BitsetError: $message';
}

/// A compact boolean array packed into 64-bit words.
///
/// Bits are numbered least-significant-bit first: bit 0 is the low bit of
/// word 0, bit 63 is its high bit, and bit 64 starts word 1. [set] and
/// [toggle] grow the logical length automatically. Bulk operations return new
/// bitsets and never mutate their operands.
///
/// Words use [BigInt] so all 64 bits behave identically on the Dart VM and in
/// JavaScript builds; each word is explicitly masked back to 64 bits.
class Bitset extends IterableBase<int> {
  Bitset([int size = 0])
      : _length = size,
        _words = List<BigInt>.filled(
          _wordsNeeded(size),
          BigInt.zero,
          growable: true,
        ) {
    if (size < 0) {
      throw RangeError.value(size, 'size', 'must be non-negative');
    }
  }

  Bitset._(this._length, List<BigInt> words)
      : _words = List<BigInt>.of(words, growable: true) {
    _cleanTrailingBits();
  }

  /// Create from a non-negative [int] or [BigInt].
  factory Bitset.fromInteger(Object value) {
    final BigInt number;
    if (value is BigInt) {
      number = value;
    } else if (value is int) {
      number = BigInt.from(value);
    } else {
      throw BitsetError('fromInteger requires an int or BigInt');
    }
    if (number.isNegative) {
      throw BitsetError(
        'fromInteger requires a non-negative value, got $value',
      );
    }
    if (number == BigInt.zero) return Bitset();

    final result = Bitset(number.bitLength);
    var remaining = number;
    for (var index = 0; index < result._words.length; index++) {
      result._words[index] = remaining & _wordMask;
      remaining >>= bitsPerWord;
    }
    return result;
  }

  /// Create from an MSB-first string containing only `0` and `1`.
  factory Bitset.fromBinaryString(String value) {
    if (!RegExp(r'^[01]*$').hasMatch(value)) {
      throw BitsetError('invalid binary string: "$value"');
    }
    final result = Bitset(value.length);
    for (var offset = 0; offset < value.length; offset++) {
      if (value[value.length - offset - 1] == '1') {
        result._setWithoutGrowth(offset);
      }
    }
    return result;
  }

  /// Short alias matching ports that call this constructor `fromBinaryStr`.
  factory Bitset.fromBinaryStr(String value) => Bitset.fromBinaryString(value);

  final List<BigInt> _words;
  int _length;

  /// Logical number of addressable bits.
  @override
  int get length => _length;

  /// Allocated bit capacity, always a multiple of 64.
  int get capacity => _words.length * bitsPerWord;

  /// Set bit [index], growing the bitset when needed.
  void set(int index) {
    _ensureCapacity(index);
    _setWithoutGrowth(index);
  }

  /// Clear bit [index]. Out-of-range positive indices are a no-op.
  void clear(int index) {
    _checkNonNegative(index);
    if (index >= _length) return;
    final wordIndex = index ~/ bitsPerWord;
    _words[wordIndex] &= _wordMask ^ _bitMask(index);
  }

  /// Return whether bit [index] is set.
  bool test(int index) {
    _checkNonNegative(index);
    if (index >= _length) return false;
    return (_words[index ~/ bitsPerWord] & _bitMask(index)) != BigInt.zero;
  }

  /// Flip bit [index], growing the bitset when needed.
  void toggle(int index) {
    _ensureCapacity(index);
    _words[index ~/ bitsPerWord] ^= _bitMask(index);
  }

  /// Intersection of this bitset and [other].
  Bitset bitwiseAnd(Bitset other) => _binaryOperation(other, (a, b) => a & b);

  /// Union of this bitset and [other].
  Bitset bitwiseOr(Bitset other) => _binaryOperation(other, (a, b) => a | b);

  /// Symmetric difference of this bitset and [other].
  Bitset bitwiseXor(Bitset other) => _binaryOperation(other, (a, b) => a ^ b);

  /// Complement of this bitset within its logical length.
  Bitset bitwiseNot() {
    final words = _words.map((word) => (~word) & _wordMask).toList();
    return Bitset._(_length, words);
  }

  /// Set difference: bits present here but not in [other].
  Bitset andNot(Bitset other) =>
      _binaryOperation(other, (a, b) => a & ((~b) & _wordMask));

  Bitset operator &(Bitset other) => bitwiseAnd(other);
  Bitset operator |(Bitset other) => bitwiseOr(other);
  Bitset operator ^(Bitset other) => bitwiseXor(other);
  Bitset operator ~() => bitwiseNot();

  /// Number of set bits.
  int get popcount {
    var count = 0;
    for (final original in _words) {
      var word = original;
      while (word != BigInt.zero) {
        word &= word - BigInt.one;
        count++;
      }
    }
    return count;
  }

  /// True when at least one bit is set.
  ///
  /// This cannot be named `any` because Dart's [Iterable.any] already accepts
  /// a predicate.
  bool get hasAny => _words.any((word) => word != BigInt.zero);

  /// True when no bits are set.
  bool get none => !hasAny;

  /// True when every logical bit is set; empty is vacuously true.
  bool get all => popcount == _length;

  /// Set-bit indices in ascending order.
  Iterable<int> get setBits sync* {
    for (var wordIndex = 0; wordIndex < _words.length; wordIndex++) {
      var word = _words[wordIndex];
      while (word != BigInt.zero) {
        final leastBit = word & -word;
        final offset = leastBit.bitLength - 1;
        final index = wordIndex * bitsPerWord + offset;
        if (index < _length) yield index;
        word &= word - BigInt.one;
      }
    }
  }

  /// Method-form alias for [setBits].
  Iterable<int> iterSetBits() => setBits;

  @override
  Iterator<int> get iterator => setBits.iterator;

  @override
  bool contains(Object? element) =>
      element is int && element >= 0 && test(element);

  /// Materialize the ascending set-bit indices.
  @override
  List<int> toList({bool growable = true}) =>
      setBits.toList(growable: growable);

  /// Convert all logical bits to one non-negative integer.
  BigInt toInteger() {
    var result = BigInt.zero;
    for (var index = _words.length - 1; index >= 0; index--) {
      result = (result << bitsPerWord) | _words[index];
    }
    return result;
  }

  /// Convert to an MSB-first binary string while preserving logical length.
  String toBinaryString() {
    if (_length == 0) return '';
    final buffer = StringBuffer();
    for (var index = _length - 1; index >= 0; index--) {
      buffer.write(test(index) ? '1' : '0');
    }
    return buffer.toString();
  }

  /// Short alias matching ports that call this conversion `toBinaryStr`.
  String toBinaryStr() => toBinaryString();

  /// Create an independent copy with the same logical length and capacity.
  Bitset copy() => Bitset._(_length, _words);

  Bitset _binaryOperation(
    Bitset other,
    BigInt Function(BigInt, BigInt) operation,
  ) {
    final resultLength = _length > other._length ? _length : other._length;
    final words = List<BigInt>.filled(_wordsNeeded(resultLength), BigInt.zero);
    for (var index = 0; index < words.length; index++) {
      final left = index < _words.length ? _words[index] : BigInt.zero;
      final right =
          index < other._words.length ? other._words[index] : BigInt.zero;
      words[index] = operation(left, right) & _wordMask;
    }
    return Bitset._(resultLength, words);
  }

  void _setWithoutGrowth(int index) {
    _words[index ~/ bitsPerWord] |= _bitMask(index);
  }

  void _ensureCapacity(int index) {
    _checkNonNegative(index);
    if (index < _length) return;
    final needed = index ~/ bitsPerWord + 1;
    if (_words.length < needed) {
      var newWordCount = _words.isEmpty ? 1 : _words.length;
      while (newWordCount < needed) {
        newWordCount *= 2;
      }
      _words.addAll(
        List<BigInt>.filled(newWordCount - _words.length, BigInt.zero),
      );
    }
    _length = index + 1;
  }

  void _cleanTrailingBits() {
    final logicalWords = _wordsNeeded(_length);
    for (var index = logicalWords; index < _words.length; index++) {
      _words[index] = BigInt.zero;
    }
    if (logicalWords == 0 || _length % bitsPerWord == 0) return;
    final used = _length % bitsPerWord;
    _words[logicalWords - 1] &= (BigInt.one << used) - BigInt.one;
  }

  static int _wordsNeeded(int bitCount) =>
      bitCount <= 0 ? 0 : (bitCount + bitsPerWord - 1) ~/ bitsPerWord;

  static BigInt _bitMask(int index) => BigInt.one << (index % bitsPerWord);

  static void _checkNonNegative(int index) {
    if (index < 0) {
      throw RangeError.value(index, 'index', 'must be non-negative');
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! Bitset || _length != other._length) return false;
    final words = _wordsNeeded(_length);
    for (var index = 0; index < words; index++) {
      if (_words[index] != other._words[index]) return false;
    }
    return true;
  }

  @override
  int get hashCode =>
      Object.hash(_length, Object.hashAll(_words.take(_wordsNeeded(_length))));

  @override
  String toString() => "Bitset('${toBinaryString()}')";
}
