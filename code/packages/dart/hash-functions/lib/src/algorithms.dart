import 'dart:convert';

final BigInt _mask32 = BigInt.parse('ffffffff', radix: 16);
final BigInt _mask64 = BigInt.parse('ffffffffffffffff', radix: 16);
final BigInt _defaultPolynomialBase = BigInt.from(31);
final BigInt _defaultPolynomialModulus = BigInt.parse('2305843009213693951');

/// FNV-1a with a 32-bit result.
int fnv1a32(List<int> data) {
  _validateBytes(data);
  var hash = BigInt.parse('811c9dc5', radix: 16);
  final prime = BigInt.parse('01000193', radix: 16);
  for (final byte in data) {
    hash = ((hash ^ BigInt.from(byte)) * prime) & _mask32;
  }
  return hash.toInt();
}

/// FNV-1a with an unsigned 64-bit result.
BigInt fnv1a64(List<int> data) {
  _validateBytes(data);
  var hash = BigInt.parse('cbf29ce484222325', radix: 16);
  final prime = BigInt.parse('100000001b3', radix: 16);
  for (final byte in data) {
    hash = ((hash ^ BigInt.from(byte)) * prime) & _mask64;
  }
  return hash;
}

/// DJB2 with unsigned 64-bit wrapping arithmetic.
BigInt djb2(List<int> data) {
  _validateBytes(data);
  var hash = BigInt.from(5381);
  for (final byte in data) {
    hash = ((hash << 5) + hash + BigInt.from(byte)) & _mask64;
  }
  return hash;
}

/// Polynomial rolling hash with the DT17 compatibility defaults.
BigInt polynomialRolling(List<int> data) => polynomialRollingWithParams(
      data,
      base: _defaultPolynomialBase,
      modulus: _defaultPolynomialModulus,
    );

/// Polynomial rolling hash using exact multiply-add before modulo reduction.
BigInt polynomialRollingWithParams(
  List<int> data, {
  required BigInt base,
  required BigInt modulus,
}) {
  _validateBytes(data);
  _validateUnsigned64(base, 'base');
  if (modulus <= BigInt.zero || modulus > _mask64) {
    throw ArgumentError('modulus must be in 1..2^64-1');
  }

  var hash = BigInt.zero;
  for (final byte in data) {
    hash = (hash * base + BigInt.from(byte)) % modulus;
  }
  return hash;
}

/// MurmurHash3 x86-32. [seed] is an unsigned 32-bit integer.
int murmur3_32(List<int> data, {int seed = 0}) {
  _validateBytes(data);
  if (seed < 0 || seed > 0xffffffff) {
    throw ArgumentError('seed must be in 0..2^32-1');
  }

  final c1 = BigInt.parse('cc9e2d51', radix: 16);
  final c2 = BigInt.parse('1b873593', radix: 16);
  var hash = BigInt.from(seed);
  var offset = 0;

  while (offset + 4 <= data.length) {
    var word = _littleEndianWord(data, offset, 4);
    word = (word * c1) & _mask32;
    word = _rotateLeft32(word, 15);
    word = (word * c2) & _mask32;

    hash = (hash ^ word) & _mask32;
    hash = _rotateLeft32(hash, 13);
    hash =
        (hash * BigInt.from(5) + BigInt.parse('e6546b64', radix: 16)) & _mask32;
    offset += 4;
  }

  final tailLength = data.length - offset;
  if (tailLength > 0) {
    var tail = _littleEndianWord(data, offset, tailLength);
    tail = (tail * c1) & _mask32;
    tail = _rotateLeft32(tail, 15);
    tail = (tail * c2) & _mask32;
    hash = (hash ^ tail) & _mask32;
  }

  hash = (hash ^ (BigInt.from(data.length) & _mask32)) & _mask32;
  hash ^= hash >> 16;
  hash = (hash * BigInt.parse('85ebca6b', radix: 16)) & _mask32;
  hash ^= hash >> 13;
  hash = (hash * BigInt.parse('c2b2ae35', radix: 16)) & _mask32;
  hash ^= hash >> 16;
  return (hash & _mask32).toInt();
}

/// SipHash-2-4 over [data] with a caller-supplied 16-byte [key].
BigInt siphash2_4(List<int> data, List<int> key) {
  _validateBytes(data);
  final copiedKey = _validatedSipHashKey(key);
  final k0 = _littleEndianWord(copiedKey, 0, 8);
  final k1 = _littleEndianWord(copiedKey, 8, 8);

  var v0 = BigInt.parse('736f6d6570736575', radix: 16) ^ k0;
  var v1 = BigInt.parse('646f72616e646f6d', radix: 16) ^ k1;
  var v2 = BigInt.parse('6c7967656e657261', radix: 16) ^ k0;
  var v3 = BigInt.parse('7465646279746573', radix: 16) ^ k1;

  var offset = 0;
  while (offset + 8 <= data.length) {
    final message = _littleEndianWord(data, offset, 8);
    v3 ^= message;
    (v0, v1, v2, v3) = _sipRounds(v0, v1, v2, v3, 2);
    v0 ^= message;
    offset += 8;
  }

  final remaining = data.length - offset;
  var last = BigInt.from(data.length & 0xff) << 56;
  for (var index = 0; index < remaining; index++) {
    last |= BigInt.from(data[offset + index]) << (index * 8);
  }

  v3 ^= last;
  (v0, v1, v2, v3) = _sipRounds(v0, v1, v2, v3, 2);
  v0 ^= last;
  v2 ^= BigInt.from(0xff);
  (v0, v1, v2, v3) = _sipRounds(v0, v1, v2, v3, 4);
  return (v0 ^ v1 ^ v2 ^ v3) & _mask64;
}

/// Hashes a Unicode string by its UTF-8 encoding with FNV-1a 32-bit.
int hashStringFnv1a32(String value) => fnv1a32(_strictUtf8Bytes(value));

/// Hashes a Unicode string by its UTF-8 encoding with SipHash-2-4.
BigInt hashStringSiphash(String value, List<int> key) =>
    siphash2_4(_strictUtf8Bytes(value), key);

List<int> _strictUtf8Bytes(String value) {
  for (var index = 0; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw ArgumentError('String must contain only Unicode scalar values');
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw ArgumentError('String must contain only Unicode scalar values');
      }
      index++;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      throw ArgumentError('String must contain only Unicode scalar values');
    }
  }
  return utf8.encode(value);
}

void _validateBytes(List<int> data) {
  for (final byte in data) {
    if (byte < 0 || byte > 255) {
      throw ArgumentError('Byte input values must be in 0..255');
    }
  }
}

void _validateUnsigned64(BigInt value, String name) {
  if (value < BigInt.zero || value > _mask64) {
    throw ArgumentError('$name must be in 0..2^64-1');
  }
}

List<int> _validatedSipHashKey(List<int> key) {
  if (key.length != 16) {
    throw ArgumentError('SipHash key must contain exactly 16 bytes');
  }
  _validateBytes(key);
  return List<int>.of(key);
}

BigInt _littleEndianWord(List<int> bytes, int offset, int count) {
  var word = BigInt.zero;
  for (var index = 0; index < count; index++) {
    word |= BigInt.from(bytes[offset + index]) << (index * 8);
  }
  return word;
}

BigInt _rotateLeft32(BigInt value, int shift) {
  final normalized = value & _mask32;
  return ((normalized << shift) | (normalized >> (32 - shift))) & _mask32;
}

BigInt _rotateLeft64(BigInt value, int shift) {
  final normalized = value & _mask64;
  return ((normalized << shift) | (normalized >> (64 - shift))) & _mask64;
}

(BigInt, BigInt, BigInt, BigInt) _sipRound(
  BigInt v0,
  BigInt v1,
  BigInt v2,
  BigInt v3,
) {
  v0 = (v0 + v1) & _mask64;
  v1 = _rotateLeft64(v1, 13) ^ v0;
  v0 = _rotateLeft64(v0, 32);
  v2 = (v2 + v3) & _mask64;
  v3 = _rotateLeft64(v3, 16) ^ v2;
  v0 = (v0 + v3) & _mask64;
  v3 = _rotateLeft64(v3, 21) ^ v0;
  v2 = (v2 + v1) & _mask64;
  v1 = _rotateLeft64(v1, 17) ^ v2;
  v2 = _rotateLeft64(v2, 32);
  return (v0, v1, v2, v3);
}

(BigInt, BigInt, BigInt, BigInt) _sipRounds(
  BigInt v0,
  BigInt v1,
  BigInt v2,
  BigInt v3,
  int count,
) {
  for (var round = 0; round < count; round++) {
    (v0, v1, v2, v3) = _sipRound(v0, v1, v2, v3);
  }
  return (v0, v1, v2, v3);
}
