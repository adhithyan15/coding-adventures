import 'algorithms.dart';

/// A typed hashing strategy with its public output width.
abstract interface class HashFunction<T> {
  T hash(List<int> data);

  int get outputBits;
}

class Fnv1a32 implements HashFunction<int> {
  const Fnv1a32();

  @override
  int hash(List<int> data) => fnv1a32(data);

  @override
  int get outputBits => 32;
}

class Fnv1a64 implements HashFunction<BigInt> {
  const Fnv1a64();

  @override
  BigInt hash(List<int> data) => fnv1a64(data);

  @override
  int get outputBits => 64;
}

class Djb2 implements HashFunction<BigInt> {
  const Djb2();

  @override
  BigInt hash(List<int> data) => djb2(data);

  @override
  int get outputBits => 64;
}

class PolynomialRolling implements HashFunction<BigInt> {
  PolynomialRolling()
      : base = BigInt.from(31),
        modulus = BigInt.parse('2305843009213693951');

  PolynomialRolling.withParams({required this.base, required this.modulus}) {
    polynomialRollingWithParams(const [], base: base, modulus: modulus);
  }

  final BigInt base;
  final BigInt modulus;

  @override
  BigInt hash(List<int> data) =>
      polynomialRollingWithParams(data, base: base, modulus: modulus);

  @override
  int get outputBits => 64;
}

class Murmur3_32 implements HashFunction<int> {
  Murmur3_32({this.seed = 0}) {
    if (seed < 0 || seed > 0xffffffff) {
      throw ArgumentError('seed must be in 0..2^32-1');
    }
  }

  final int seed;

  @override
  int hash(List<int> data) => murmur3_32(data, seed: seed);

  @override
  int get outputBits => 32;
}

class SipHash24 implements HashFunction<BigInt> {
  SipHash24(List<int> key) : _key = List<int>.unmodifiable(_copyKey(key));

  final List<int> _key;

  @override
  BigInt hash(List<int> data) => siphash2_4(data, _key);

  @override
  int get outputBits => 64;

  static List<int> _copyKey(List<int> key) {
    if (key.length != 16) {
      throw ArgumentError('SipHash key must contain exactly 16 bytes');
    }
    for (final byte in key) {
      if (byte < 0 || byte > 255) {
        throw ArgumentError('Byte input values must be in 0..255');
      }
    }
    return List<int>.of(key);
  }
}
