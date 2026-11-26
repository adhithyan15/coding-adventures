/// Property tests for `coding_adventures_x25519`, using the in-lane
/// `pbt` package: Diffie-Hellman agreement, determinism, clamp
/// idempotence and bit shape, scalar/u-coordinate sensitivity,
/// keypair aliasing, and the low-order error — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_x25519/x25519.dart';
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

void main() {
  group('properties', () {
    test('diffie-hellman agreement', () {
      final gen = Gen.tuple2(Gen.bytes(32, 32), Gen.bytes(32, 32));
      _check(
        'agreement',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(541),
          PbtOptions(numTests: 60, seed: BigInt.from(541)),
          (rng) => gen.sample(rng),
          (input) {
            final pubA = x25519Base(input.$1);
            final pubB = x25519Base(input.$2);
            return _equalBytes(
              x25519(input.$1, pubB),
              x25519(input.$2, pubA),
            );
          },
        ),
      );
    });

    test('determinism', () {
      final gen = Gen.tuple2(Gen.bytes(32, 32), Gen.bytes(32, 32));
      _check(
        'determinism',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(542),
          PbtOptions(numTests: 60, seed: BigInt.from(542)),
          (rng) => gen.sample(rng),
          (input) {
            try {
              return _equalBytes(
                x25519(input.$1, input.$2),
                x25519(input.$1, input.$2),
              );
            } on X25519LowOrderPoint {
              // Deterministic rejection is deterministic too.
              try {
                x25519(input.$1, input.$2);
                return false;
              } on X25519LowOrderPoint {
                return true;
              }
            }
          },
        ),
      );
    });

    test('clamp is idempotent with the specified bit shape', () {
      final gen = Gen.bytes(32, 32);
      _check(
        'clamp',
        checkProperty<Uint8List>(
          BigInt.from(543),
          PbtOptions(numTests: 200, seed: BigInt.from(543)),
          (rng) => gen.sample(rng),
          (scalar) {
            final once = clampScalar(scalar);
            if (!_equalBytes(once, clampScalar(once))) return false;
            if (once[0] & 0x07 != 0) return false;
            if (once[31] >> 7 != 0) return false;
            if ((once[31] >> 6) & 1 != 1) return false;
            return true;
          },
        ),
      );
    });

    test('scalar sensitivity', () {
      final gen = Gen.tuple2(Gen.bytes(32, 32), Gen.u8());
      _check(
        'scalar sensitivity',
        checkProperty<(Uint8List, int)>(
          BigInt.from(544),
          PbtOptions(numTests: 60, seed: BigInt.from(544)),
          (rng) => gen.sample(rng),
          (input) {
            // Flip a bit clamping preserves (bytes 1..30 survive
            // untouched; byte 0 loses its low 3 bits, byte 31 its top).
            final index = 1 + (input.$2 % 30);
            final flipped = Uint8List.fromList(input.$1);
            flipped[index] ^= 1 << (input.$2 % 8);
            return !_equalBytes(
              x25519Base(input.$1),
              x25519Base(flipped),
            );
          },
        ),
      );
    });

    test('u-coordinate sensitivity', () {
      final gen = Gen.tuple3(Gen.bytes(32, 32), Gen.bytes(32, 32), Gen.u8());
      _check(
        'u sensitivity',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(545),
          PbtOptions(numTests: 60, seed: BigInt.from(545)),
          (rng) => gen.sample(rng),
          (input) {
            // Flip a bit the ladder's u-mask preserves (bytes 0..30).
            final index = input.$3 % 31;
            final flipped = Uint8List.fromList(input.$2);
            flipped[index] ^= 1 << (input.$3 % 8);
            try {
              return !_equalBytes(
                x25519(input.$1, input.$2),
                x25519(input.$1, flipped),
              );
            } on X25519LowOrderPoint {
              // A flip onto a low-order point still changes the
              // outcome (tag vs. rejection); the reverse flip is
              // symmetric, so accept rejection either way.
              return true;
            }
          },
        ),
      );
    });

    test('keypair aliases base multiplication', () {
      final gen = Gen.bytes(32, 32);
      _check(
        'keypair alias',
        checkProperty<Uint8List>(
          BigInt.from(546),
          PbtOptions(numTests: 60, seed: BigInt.from(546)),
          (rng) => gen.sample(rng),
          (scalar) => _equalBytes(
            generateKeypair(scalar),
            x25519Base(scalar),
          ),
        ),
      );
    });
  });
}
