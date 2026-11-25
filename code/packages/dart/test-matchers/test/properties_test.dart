/// Property tests for `coding_adventures_test_matchers`, using the in-lane
/// `pbt` package: universal claims over generated inputs, each with a fixed
/// seed so failures reproduce exactly.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_test_matchers/coding_adventures_test_matchers.dart'
    hide expect;
import 'package:test/test.dart';

/// Generates nested values up to [depth] deep: nulls, bools, ints, arbitrary
/// doubles (via raw bits, so NaN payloads, infinities, and subnormals all
/// appear), strings, byte arrays, lists, and maps.
Gen<Object?> valueGen(int depth) {
  final leaves = <Gen<Object?>>[
    Gen.constant(null),
    Gen.boolean(),
    Gen.u8(),
    Gen.integer(BigInt.from(-1000), BigInt.from(1000)).map((b) => b.toInt()),
    Gen.u64().map(f64BitsToNumber),
    Gen.bytes(0, 8).map(String.fromCharCodes),
    Gen.bytes(0, 16),
  ];
  if (depth <= 0) return Gen.oneOf(leaves);
  return Gen.oneOf([
    ...leaves,
    Gen.list(0, 4, valueGen(depth - 1)),
    Gen.list(0, 4, Gen.tuple2(Gen.u8(), valueGen(depth - 1))).map(
        (entries) => Map.fromEntries(entries.map((e) => MapEntry(e.$1, e.$2)))),
  ]);
}

/// Fails the test with the counter-example when [verdict] is a [Fail].
void checkVerdict(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('deepEqual is reflexive', () {
      final gen = valueGen(3);
      final verdict = checkProperty<Object?>(
        BigInt.from(11),
        PbtOptions(numTests: 200, seed: BigInt.from(11)),
        (rng) => gen.sample(rng),
        (v) => deepEqual(v, v),
      );
      checkVerdict('deepEqual reflexive', verdict);
    });

    test('deepEqual is symmetric', () {
      final gen = Gen.tuple2(valueGen(3), valueGen(3));
      final verdict = checkProperty<(Object?, Object?)>(
        BigInt.from(12),
        PbtOptions(numTests: 200, seed: BigInt.from(12)),
        (rng) => gen.sample(rng),
        (pair) => deepEqual(pair.$1, pair.$2) == deepEqual(pair.$2, pair.$1),
      );
      checkVerdict('deepEqual symmetric', verdict);
    });

    test('f64 bit conversion round-trips bit-exactly', () {
      final verdict = checkProperty<BigInt>(
        BigInt.from(13),
        PbtOptions(numTests: 500, seed: BigInt.from(13)),
        (rng) => Gen.u64().sample(rng),
        (bits) => numberToF64Bits(f64BitsToNumber(bits)) == bits,
      );
      checkVerdict('f64 round-trip', verdict);
    });

    test('canonicalF64 is idempotent', () {
      final verdict = checkProperty<BigInt>(
        BigInt.from(14),
        PbtOptions(numTests: 500, seed: BigInt.from(14)),
        (rng) => Gen.u64().sample(rng),
        (bits) => canonicalF64(canonicalF64(bits)) == canonicalF64(bits),
      );
      checkVerdict('canonicalF64 idempotent', verdict);
    });

    test('canonicalF64 fixes exactly NaN and -0.0', () {
      final quietNaN = BigInt.parse('7ff8000000000000', radix: 16);
      final verdict = checkProperty<BigInt>(
        BigInt.from(15),
        PbtOptions(numTests: 500, seed: BigInt.from(15)),
        (rng) => Gen.u64().sample(rng),
        (bits) {
          final exponent = (bits >> 52) & BigInt.from(0x7ff);
          final mantissa = bits & ((BigInt.one << 52) - BigInt.one);
          final isNaN =
              exponent == BigInt.from(0x7ff) && mantissa != BigInt.zero;
          final canonical = canonicalF64(bits);
          if (isNaN) return canonical == quietNaN;
          if (bits == BigInt.one << 63) return canonical == BigInt.zero;
          return canonical == bits;
        },
      );
      checkVerdict('canonicalF64 exact', verdict);
    });

    test('drawn elements are contained with the right length', () {
      final gen = Gen.list(1, 8, Gen.u8());
      final verdict = checkProperty<List<int>>(
        BigInt.from(16),
        PbtOptions(numTests: 200, seed: BigInt.from(16)),
        (rng) => gen.sample(rng),
        (list) {
          try {
            Expectation(list).toContain(list.first);
            Expectation(list).toHaveLength(list.length);
            return true;
          } on AssertionError {
            return false;
          }
        },
      );
      checkVerdict('membership', verdict);
    });

    test('map equality is order-insensitive', () {
      final gen = Gen.list(0, 6, Gen.u8());
      final verdict = checkProperty<List<int>>(
        BigInt.from(17),
        PbtOptions(numTests: 200, seed: BigInt.from(17)),
        (rng) => gen.sample(rng),
        (values) {
          // Index keys are unique by construction, so forward and reversed
          // hold the same entries (duplicate keys would legitimately differ
          // under last-wins collapse).
          final entries =
              values.asMap().entries.map((e) => MapEntry(e.key, e.value));
          return deepEqual(
            Map.fromEntries(entries),
            Map.fromEntries(entries.toList().reversed),
          );
        },
      );
      checkVerdict('map order-insensitive', verdict);
    });

    test('encodeValue is total and never empty', () {
      final gen = valueGen(3);
      final verdict = checkProperty<Object?>(
        BigInt.from(18),
        PbtOptions(numTests: 200, seed: BigInt.from(18)),
        (rng) => gen.sample(rng),
        (v) => encodeValue(toValue(v)).isNotEmpty,
      );
      checkVerdict('encodeValue total', verdict);
    });
  });
}
