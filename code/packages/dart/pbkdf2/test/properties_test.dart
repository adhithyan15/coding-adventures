/// Property tests for `coding_adventures_pbkdf2`, using the in-lane
/// `pbt` package: determinism, input sensitivity, truncation and
/// multi-block consistency, empty-salt acceptance, the allow-flag
/// contract, and the validation matrix — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbkdf2/pbkdf2.dart';
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

Uint8List _derive(
  int variant,
  Uint8List password,
  Uint8List salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) =>
    switch (variant % 3) {
      0 => pbkdf2HmacSha1(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
      1 => pbkdf2HmacSha256(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
      _ => pbkdf2HmacSha512(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
    };

void main() {
  group('properties', () {
    test('determinism on every variant', () {
      final gen = Gen.tuple3(Gen.bytes(1, 32), Gen.bytes(0, 32), Gen.u8());
      _check(
        'determinism',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(581),
          PbtOptions(numTests: 60, seed: BigInt.from(581)),
          (rng) => gen.sample(rng),
          (input) {
            final iterations = 1 + (input.$3 % 10);
            final keyLength = 1 + ((input.$3 >> 3) % 96);
            for (var variant = 0; variant < 3; variant++) {
              final a = _derive(variant, input.$1, input.$2,
                  iterations: iterations, keyLength: keyLength);
              final b = _derive(variant, input.$1, input.$2,
                  iterations: iterations, keyLength: keyLength);
              if (!_equalBytes(a, b) || a.length != keyLength) return false;
            }
            return true;
          },
        ),
      );
    });

    test('every input binds the key', () {
      final gen = Gen.tuple3(Gen.bytes(1, 24), Gen.bytes(1, 24), Gen.u8());
      _check(
        'sensitivity',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(582),
          PbtOptions(numTests: 40, seed: BigInt.from(582)),
          (rng) => gen.sample(rng),
          (input) {
            Uint8List flip(Uint8List v) {
              final out = Uint8List.fromList(v);
              out[input.$3 % out.length] ^= 1 << (input.$3 % 8);
              return out;
            }

            final base =
                _derive(1, input.$1, input.$2, iterations: 2, keyLength: 32);
            if (_equalBytes(
                base,
                _derive(1, flip(input.$1), input.$2,
                    iterations: 2, keyLength: 32))) {
              return false;
            }
            if (_equalBytes(
                base,
                _derive(1, input.$1, flip(input.$2),
                    iterations: 2, keyLength: 32))) {
              return false;
            }
            if (_equalBytes(base,
                _derive(1, input.$1, input.$2, iterations: 3, keyLength: 32))) {
              return false;
            }
            return true;
          },
        ),
      );
    });

    test('shorter keys prefix longer keys', () {
      final gen = Gen.tuple3(Gen.bytes(1, 24), Gen.bytes(0, 24), Gen.u8());
      _check(
        'truncation',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(583),
          PbtOptions(numTests: 40, seed: BigInt.from(583)),
          (rng) => gen.sample(rng),
          (input) {
            final iterations = 1 + (input.$3 % 5);
            final shortLen = 1 + (input.$3 % 32);
            final longLen = shortLen + 1 + ((input.$3 >> 2) % 64);
            for (var variant = 0; variant < 3; variant++) {
              final short = _derive(variant, input.$1, input.$2,
                  iterations: iterations, keyLength: shortLen);
              final long = _derive(variant, input.$1, input.$2,
                  iterations: iterations, keyLength: longLen);
              if (!_equalBytes(short, long.sublist(0, shortLen))) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('empty password needs the flag, then works', () {
      final gen = Gen.tuple2(Gen.bytes(0, 24), Gen.u8());
      _check(
        'allow flag',
        checkProperty<(Uint8List, int)>(
          BigInt.from(584),
          PbtOptions(numTests: 40, seed: BigInt.from(584)),
          (rng) => gen.sample(rng),
          (input) {
            final empty = Uint8List(0);
            for (var variant = 0; variant < 3; variant++) {
              try {
                _derive(variant, empty, input.$1, iterations: 1, keyLength: 16);
                return false;
              } on Pbkdf2EmptyPassword {
                // Expected.
              }
              final dk = _derive(variant, empty, input.$1,
                  iterations: 1, keyLength: 16, allowEmptyPassword: true);
              if (dk.length != 16) return false;
            }
            return true;
          },
        ),
      );
    });

    test('validation rejects exactly the bad shapes', () {
      final gen = Gen.u8();
      _check(
        'validation matrix',
        checkProperty<int>(
          BigInt.from(585),
          PbtOptions(numTests: 60, seed: BigInt.from(585)),
          (rng) => gen.sample(rng),
          (pick) {
            final password = Uint8List.fromList([1, 2, 3]);
            final salt = Uint8List.fromList([4, 5]);
            for (var variant = 0; variant < 3; variant++) {
              for (final bad in [0, -(pick % 50 + 1)]) {
                try {
                  _derive(variant, password, salt,
                      iterations: bad, keyLength: 16);
                } on Pbkdf2InvalidIterations {
                  continue;
                }
                return false;
              }
              for (final bad in [0, -(pick % 50 + 1)]) {
                try {
                  _derive(variant, password, salt,
                      iterations: 1, keyLength: bad);
                } on Pbkdf2InvalidKeyLength {
                  continue;
                }
                return false;
              }
              try {
                _derive(variant, password, salt,
                    iterations: 1, keyLength: (1 << 20) + 1);
              } on Pbkdf2KeyLengthTooLarge {
                continue;
              }
              return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
