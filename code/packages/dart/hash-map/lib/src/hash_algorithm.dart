part of coding_adventures_hash_map;

sealed class HashAlgorithm {
  const HashAlgorithm._();

  const factory HashAlgorithm.fnv1a32() = _Fnv1a32Algorithm;
  factory HashAlgorithm.murmur3({required int seed}) =>
      _Murmur3Algorithm(seed: seed);
  factory HashAlgorithm.sipHash24({required List<int> key}) =>
      _SipHash24Algorithm(key: key);

  BigInt validatedHash(List<int> bytes) {
    _validateConfiguration();
    return _hashValidated(bytes);
  }

  BigInt _hashValidated(List<int> bytes) {
    _validateByteValues(bytes);
    if (this is _Fnv1a32Algorithm) return BigInt.from(fnv1a32(bytes));
    if (this is _Murmur3Algorithm) {
      final algorithm = this as _Murmur3Algorithm;
      return BigInt.from(murmur3_32(bytes, seed: algorithm.seed));
    }
    return siphash2_4(bytes, (this as _SipHash24Algorithm).key);
  }

  void _validateConfiguration() {
    if (this is _Murmur3Algorithm) {
      final seed = (this as _Murmur3Algorithm).seed;
      if (seed < 0 || seed > 0xffffffff) throw InvalidMurmurSeed(seed);
    } else if (this is _SipHash24Algorithm) {
      final key = (this as _SipHash24Algorithm).key;
      if (key.length != 16) throw InvalidSipHashKeyLength(key.length);
      _validateByteValues(key);
    }
  }
}

final class _Fnv1a32Algorithm extends HashAlgorithm {
  const _Fnv1a32Algorithm() : super._();

  @override
  bool operator ==(Object other) => other is _Fnv1a32Algorithm;

  @override
  int get hashCode => Object.hash(_Fnv1a32Algorithm, 0);
}

final class _Murmur3Algorithm extends HashAlgorithm {
  const _Murmur3Algorithm({required this.seed}) : super._();

  final int seed;

  @override
  bool operator ==(Object other) =>
      other is _Murmur3Algorithm && other.seed == seed;

  @override
  int get hashCode => Object.hash(_Murmur3Algorithm, seed);
}

final class _SipHash24Algorithm extends HashAlgorithm {
  _SipHash24Algorithm({required List<int> key})
      : key = List<int>.unmodifiable(List<int>.of(key)),
        super._();

  final List<int> key;

  @override
  bool operator ==(Object other) =>
      other is _SipHash24Algorithm && _sameBytes(other.key, key);

  @override
  int get hashCode => Object.hash(_SipHash24Algorithm, Object.hashAll(key));
}

bool _sameBytes(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

void _validateByteValues(List<int> bytes) {
  for (var index = 0; index < bytes.length; index++) {
    final value = bytes[index];
    if (value < 0 || value > 255) throw InvalidByte(index, value);
  }
}
