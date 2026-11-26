/// Property tests for `coding_adventures_hkdf`, using the in-lane
/// `pbt` package: extract/expand round-trips, determinism, input
/// sensitivity, fixed-vs-legacy agreement, length exactness and the
/// error matrix, default-salt equivalence, and import round-trips —
/// all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_hkdf/hkdf.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

const _algos = [
  HashAlgorithm.sha256,
  HashAlgorithm.sha512,
  HashAlgorithm.blake2b
];

/// Flat 4-tuple generator over the lane's tuple2/tuple3 primitives.
Gen<(A, B, C, D)> _tuple4<A, B, C, D>(
  Gen<A> a,
  Gen<B> b,
  Gen<C> c,
  Gen<D> d,
) =>
    Gen.tuple2(Gen.tuple3(a, b, c), d)
        .map((t) => (t.$1.$1, t.$1.$2, t.$1.$3, t.$2));

void main() {
  group('properties', () {
    test('round trip equals combined on every algorithm', () {
      final gen = _tuple4(
        Gen.bytes(0, 64),
        Gen.bytes(1, 64),
        Gen.bytes(0, 64),
        Gen.u8(),
      );
      _check(
        'round trip',
        checkProperty<(Uint8List, Uint8List, Uint8List, int)>(
          BigInt.from(561),
          PbtOptions(numTests: 60, seed: BigInt.from(561)),
          (rng) => gen.sample(rng),
          (input) {
            final length = 1 + (input.$4 % 200);
            for (final algo in _algos) {
              final combined = hkdf(input.$1, input.$2, input.$3, length, algo);
              final prk = hkdfExtract(input.$1, input.$2, algo);
              if (prk.length != algo.hashLen) return false;
              final manual = hkdfExpand(prk, input.$3, length, algo);
              if (!_equalBytes(combined, manual)) return false;
            }
            return true;
          },
        ),
      );
    });

    test('determinism on every algorithm', () {
      final gen = _tuple4(
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.u8(),
      );
      _check(
        'determinism',
        checkProperty<(Uint8List, Uint8List, Uint8List, int)>(
          BigInt.from(562),
          PbtOptions(numTests: 60, seed: BigInt.from(562)),
          (rng) => gen.sample(rng),
          (input) {
            final length = 1 + (input.$4 % 200);
            for (final algo in _algos) {
              final a = hkdf(input.$1, input.$2, input.$3, length, algo);
              final b = hkdf(input.$1, input.$2, input.$3, length, algo);
              if (!_equalBytes(a, b) || a.length != length) return false;
            }
            return true;
          },
        ),
      );
    });

    test('every input binds the output', () {
      final gen = _tuple4(
        Gen.bytes(1, 48),
        Gen.bytes(1, 48),
        Gen.bytes(1, 48),
        Gen.u8(),
      );
      _check(
        'sensitivity',
        checkProperty<(Uint8List, Uint8List, Uint8List, int)>(
          BigInt.from(563),
          PbtOptions(numTests: 60, seed: BigInt.from(563)),
          (rng) => gen.sample(rng),
          (input) {
            const length = 64;
            Uint8List flip(Uint8List v) {
              final out = Uint8List.fromList(v);
              out[input.$4 % out.length] ^= 1 << (input.$4 % 8);
              return out;
            }

            for (final algo in [HashAlgorithm.sha256, HashAlgorithm.blake2b]) {
              final base = hkdf(input.$1, input.$2, input.$3, length, algo);
              if (_equalBytes(base,
                  hkdf(flip(input.$1), input.$2, input.$3, length, algo))) {
                return false;
              }
              if (_equalBytes(base,
                  hkdf(input.$1, flip(input.$2), input.$3, length, algo))) {
                return false;
              }
              if (_equalBytes(base,
                  hkdf(input.$1, input.$2, flip(input.$3), length, algo))) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('fixed sha256 agrees with legacy', () {
      final gen = _tuple4(
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.u8(),
      );
      _check(
        'fixed agreement',
        checkProperty<(Uint8List, Uint8List, Uint8List, int)>(
          BigInt.from(564),
          PbtOptions(numTests: 60, seed: BigInt.from(564)),
          (rng) => gen.sample(rng),
          (input) {
            final length = 1 + (input.$4 % 200);
            final prk =
                hkdfExtractFixed(input.$1, input.$2, FixedHkdfHash.sha256);
            try {
              final legacyPrk =
                  hkdfExtract(input.$1, input.$2, HashAlgorithm.sha256);
              if (!_equalBytes(prk.asBytes(), legacyPrk)) return false;
              final fixedOut = Uint8List(length);
              hkdfExpandFixed(prk, input.$3, fixedOut);
              final legacyOut =
                  hkdfExpand(legacyPrk, input.$3, length, HashAlgorithm.sha256);
              return _equalBytes(fixedOut, legacyOut);
            } finally {
              prk.dispose();
            }
          },
        ),
      );
    });

    test('fixed sha384 lengths and determinism', () {
      final gen = _tuple4(
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.bytes(0, 64),
        Gen.u8(),
      );
      _check(
        'fixed sha384',
        checkProperty<(Uint8List, Uint8List, Uint8List, int)>(
          BigInt.from(565),
          PbtOptions(numTests: 40, seed: BigInt.from(565)),
          (rng) => gen.sample(rng),
          (input) {
            final length = 1 + (input.$4 % 200);
            final prk =
                hkdfExtractFixed(input.$1, input.$2, FixedHkdfHash.sha384);
            try {
              if (prk.asBytes().length != 48) return false;
              final a = Uint8List(length);
              final b = Uint8List(length);
              hkdfExpandFixed(prk, input.$3, a);
              hkdfExpandFixed(prk, input.$3, b);
              return _equalBytes(a, b);
            } finally {
              prk.dispose();
            }
          },
        ),
      );
    });

    test('length matrix rejects exactly the bad shapes', () {
      final gen = Gen.u8();
      _check(
        'length matrix',
        checkProperty<int>(
          BigInt.from(566),
          PbtOptions(numTests: 60, seed: BigInt.from(566)),
          (rng) => gen.sample(rng),
          (pick) {
            final prk = Uint8List.fromList(List<int>.filled(64, 0x01));
            final info = Uint8List.fromList([9]);
            for (final algo in _algos) {
              final maximum = 255 * algo.hashLen;
              // Zero and negative always reject.
              for (final bad in [0, -1, -(pick % 100 + 1)]) {
                try {
                  hkdfExpand(prk, info, bad, algo);
                  return false;
                } on HkdfOutputTooShort {
                  continue;
                }
              }
              // Over-maximum rejects with exact payloads.
              final over = maximum + 1 + (pick % 100);
              try {
                hkdfExpand(prk, info, over, algo);
                return false;
              } on HkdfOutputTooLong catch (e) {
                if (e.requested != over || e.maximum != maximum) {
                  return false;
                }
              }
              // Boundary and interior lengths succeed exactly.
              for (final good in [1, algo.hashLen, maximum]) {
                if (hkdfExpand(prk, info, good, algo).length != good) {
                  return false;
                }
              }
            }
            return true;
          },
        ),
      );
    });

    test('empty salt equals zero salt on every algorithm', () {
      final gen = Gen.tuple2(Gen.bytes(0, 64), Gen.bytes(0, 64));
      _check(
        'default salt',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(567),
          PbtOptions(numTests: 60, seed: BigInt.from(567)),
          (rng) => gen.sample(rng),
          (input) {
            for (final algo in _algos) {
              final zeros = Uint8List(algo.hashLen);
              if (!_equalBytes(hkdfExtract(Uint8List(0), input.$1, algo),
                  hkdfExtract(zeros, input.$1, algo))) {
                return false;
              }
              if (!_equalBytes(hkdf(Uint8List(0), input.$1, input.$2, 48, algo),
                  hkdf(zeros, input.$1, input.$2, 48, algo))) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('import round-trips through expand', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 48),
        Gen.bytes(0, 48),
        Gen.bytes(0, 48),
      );
      _check(
        'import round trip',
        checkProperty<(Uint8List, Uint8List, Uint8List)>(
          BigInt.from(568),
          PbtOptions(numTests: 40, seed: BigInt.from(568)),
          (rng) => gen.sample(rng),
          (input) {
            for (final algorithm in [
              FixedHkdfHash.sha256,
              FixedHkdfHash.sha384
            ]) {
              final extracted = hkdfExtractFixed(input.$1, input.$2, algorithm);
              try {
                final imported =
                    HkdfPrk.importExact(algorithm, extracted.asBytes());
                try {
                  final a = Uint8List(80);
                  final b = Uint8List(80);
                  hkdfExpandFixed(extracted, input.$3, a);
                  hkdfExpandFixed(imported, input.$3, b);
                  if (!_equalBytes(a, b)) return false;
                } finally {
                  imported.dispose();
                }
              } finally {
                extracted.dispose();
              }
            }
            return true;
          },
        ),
      );
    });
  });
}
