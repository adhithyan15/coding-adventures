import 'package:coding_adventures_hash_functions/coding_adventures_hash_functions.dart';
import 'package:test/test.dart';

const _mask32 = 0xffffffff;
final _mask64 = (BigInt.one << 64) - BigInt.one;

/// SplitMix64 uses only BigInt so the test seed and stream are identical on VM
/// and Node. This is deterministic generated testing, not coverage-guided fuzz.
class _SplitMix64 {
  _SplitMix64(BigInt seed) : _state = seed & _mask64;

  BigInt _state;

  BigInt next64() {
    _state = (_state + BigInt.parse('9e3779b97f4a7c15', radix: 16)) & _mask64;
    var value = _state;
    value = ((value ^ (value >> 30)) *
            BigInt.parse('bf58476d1ce4e5b9', radix: 16)) &
        _mask64;
    value = ((value ^ (value >> 27)) *
            BigInt.parse('94d049bb133111eb', radix: 16)) &
        _mask64;
    return value ^ (value >> 31);
  }

  int nextInt(int maxExclusive) =>
      (next64() % BigInt.from(maxExclusive)).toInt();
}

BigInt _referenceFnv64(List<int> data) {
  var value = BigInt.parse('cbf29ce484222325', radix: 16);
  final prime = BigInt.parse('100000001b3', radix: 16);
  for (final byte in data) {
    value = ((value ^ BigInt.from(byte)) * prime) & _mask64;
  }
  return value;
}

int _referenceFnv32(List<int> data) {
  var value = BigInt.parse('811c9dc5', radix: 16);
  final prime = BigInt.parse('1000193', radix: 16);
  final modulus = BigInt.one << 32;
  for (final byte in data) {
    value = ((value ^ BigInt.from(byte)) * prime) % modulus;
  }
  return value.toInt();
}

BigInt _referenceDjb2(List<int> data) {
  var value = BigInt.from(5381);
  for (final byte in data) {
    value = ((value * BigInt.from(33)) + BigInt.from(byte)) & _mask64;
  }
  return value;
}

BigInt _referencePolynomial(List<int> data, BigInt base, BigInt modulus) {
  var value = BigInt.zero;
  for (final byte in data) {
    value = (value * base + BigInt.from(byte)) % modulus;
  }
  return value;
}

void main() {
  test(
      'seeded long-input properties and one-bit mutation history '
      '(seed=0x445431375f70726f70, cases=192, length=0..512)', () {
    final random = _SplitMix64(BigInt.parse('445431375f70726f70', radix: 16));
    final defaultModulus = BigInt.parse('2305843009213693951');

    for (var caseIndex = 0; caseIndex < 192; caseIndex++) {
      final length = random.nextInt(513);
      final data = List<int>.generate(length, (_) => random.nextInt(256));
      final original = List<int>.of(data);
      final seed = random.nextInt(0x100000000);
      final key = List<int>.generate(16, (_) => random.nextInt(256));
      final keySnapshot = List<int>.of(key);
      final base = random.next64();
      final generatedModulus = random.next64();
      final modulus =
          generatedModulus == BigInt.zero ? BigInt.one : generatedModulus;

      final fnv32 = fnv1a32(data);
      final fnv64 = fnv1a64(data);
      final djb = djb2(data);
      final polyDefault = polynomialRolling(data);
      final polyCustom = polynomialRollingWithParams(
        data,
        base: base,
        modulus: modulus,
      );
      final murmur = murmur3_32(data, seed: seed);
      final sip = siphash2_4(data, key);

      expect(fnv32, _referenceFnv32(data), reason: 'case=$caseIndex fnv32');
      expect(fnv64, _referenceFnv64(data), reason: 'case=$caseIndex fnv64');
      expect(djb, _referenceDjb2(data), reason: 'case=$caseIndex djb2');
      expect(
        polyDefault,
        _referencePolynomial(data, BigInt.from(31), defaultModulus),
        reason: 'case=$caseIndex polynomial-default',
      );
      expect(
        polyCustom,
        _referencePolynomial(data, base, modulus),
        reason: 'case=$caseIndex polynomial-params',
      );
      expect(fnv32 >= 0 && fnv32 <= _mask32, isTrue,
          reason: 'case=$caseIndex fnv32-range');
      expect(fnv64 >= BigInt.zero && fnv64 <= _mask64, isTrue,
          reason: 'case=$caseIndex fnv64-range');
      expect(djb >= BigInt.zero && djb <= _mask64, isTrue,
          reason: 'case=$caseIndex djb2-range');
      expect(murmur >= 0 && murmur <= _mask32, isTrue,
          reason: 'case=$caseIndex murmur-range');
      expect(sip >= BigInt.zero && sip <= _mask64, isTrue,
          reason: 'case=$caseIndex sip-range');

      // Interleave the same operations and wrappers to catch hidden state and
      // accidental mutation of caller-owned input/key lists.
      expect(fnv1a32(data), fnv32, reason: 'case=$caseIndex fnv32-repeat');
      expect(Murmur3_32(seed: seed).hash(data), murmur,
          reason: 'case=$caseIndex murmur-wrapper');
      expect(SipHash24(key).hash(data), sip,
          reason: 'case=$caseIndex sip-wrapper');
      expect(data, original, reason: 'case=$caseIndex input-ownership');
      expect(key, keySnapshot, reason: 'case=$caseIndex key-ownership');

      if (data.isNotEmpty) {
        final changed = List<int>.of(data);
        final byteIndex = random.nextInt(changed.length);
        changed[byteIndex] ^= 1 << random.nextInt(8);
        final changedBeforeCall = List<int>.of(changed);
        // A mutation may collide for some hash functions; the property here
        // is that each mutated input remains independently processable and
        // does not alter either the original or the supplied mutation.
        final changedHash = fnv1a64(changed);
        expect(changedHash, _referenceFnv64(changed),
            reason: 'case=$caseIndex mutated-fnv64');
        expect(changed, changedBeforeCall,
            reason: 'case=$caseIndex mutated-input-ownership');
        expect(data, original,
            reason: 'case=$caseIndex original-after-mutation');
      }
    }
  });

  test(
      'strategy wrappers remain consistent across deterministic call sequences',
      () {
    final random = _SplitMix64(BigInt.parse('445431375f77726170', radix: 16));
    final fnv32 = Fnv1a32();
    final fnv64 = Fnv1a64();
    final djb = Djb2();
    final polynomial = PolynomialRolling();
    final murmur = Murmur3_32(seed: 0x9747b28c);
    final key = List<int>.generate(16, (_) => random.nextInt(256));
    final sip = SipHash24(key);

    for (var callIndex = 0; callIndex < 96; callIndex++) {
      final data = List<int>.generate(
        random.nextInt(257),
        (_) => random.nextInt(256),
      );
      expect(fnv32.hash(data), fnv1a32(data), reason: 'call=$callIndex fnv32');
      expect(fnv64.hash(data), fnv1a64(data), reason: 'call=$callIndex fnv64');
      expect(djb.hash(data), djb2(data), reason: 'call=$callIndex djb2');
      expect(polynomial.hash(data), polynomialRolling(data),
          reason: 'call=$callIndex polynomial');
      expect(
        murmur.hash(data),
        murmur3_32(data, seed: 0x9747b28c),
        reason: 'call=$callIndex murmur',
      );
      expect(sip.hash(data), siphash2_4(data, key),
          reason: 'call=$callIndex siphash');
    }
  });
}
