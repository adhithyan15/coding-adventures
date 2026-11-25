/// Property tests for `coding_adventures_scrypt`, using the in-lane
/// `pbt` package: determinism, input and parameter sensitivity,
/// output lengths, empty-input acceptance, and the validation
/// matrix — all with fixed seeds. Cost parameters stay small (N ≤
/// 64); the heavy RFC vectors live in the unit suite.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_scrypt/scrypt.dart';
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
    test('determinism and exact lengths', () {
      final gen = Gen.tuple3(Gen.bytes(0, 32), Gen.bytes(0, 32), Gen.u8());
      _check(
        'determinism',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(621),
          PbtOptions(numTests: 40, seed: BigInt.from(621)),
          (rng) => gen.sample(rng),
          (input) {
            final n = 1 << (1 + (input.$3 % 6));
            final r = 1 + ((input.$3 >> 3) % 3);
            final dkLen = 1 + ((input.$3 >> 5) % 96);
            final a =
                scrypt(input.$1, input.$2, n: n, r: r, p: 1, dkLen: dkLen);
            final b =
                scrypt(input.$1, input.$2, n: n, r: r, p: 1, dkLen: dkLen);
            return _equalBytes(a, b) && a.length == dkLen;
          },
        ),
      );
    });

    test('every input binds the key', () {
      final gen = Gen.tuple3(Gen.bytes(1, 24), Gen.bytes(1, 24), Gen.u8());
      _check(
        'sensitivity',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(622),
          PbtOptions(numTests: 30, seed: BigInt.from(622)),
          (rng) => gen.sample(rng),
          (input) {
            Uint8List flip(Uint8List v) {
              final out = Uint8List.fromList(v);
              out[input.$3 % out.length] ^= 1 << (input.$3 % 8);
              return out;
            }

            final base =
                scrypt(input.$1, input.$2, n: 16, r: 1, p: 1, dkLen: 32);
            if (_equalBytes(
                base,
                scrypt(flip(input.$1), input.$2,
                    n: 16, r: 1, p: 1, dkLen: 32))) {
              return false;
            }
            if (_equalBytes(
                base,
                scrypt(input.$1, flip(input.$2),
                    n: 16, r: 1, p: 1, dkLen: 32))) {
              return false;
            }
            if (_equalBytes(base,
                scrypt(input.$1, input.$2, n: 32, r: 1, p: 1, dkLen: 32))) {
              return false;
            }
            return true;
          },
        ),
      );
    });

    test('validation rejects exactly the bad shapes', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'validation matrix',
        checkProperty<(int, int)>(
          BigInt.from(623),
          PbtOptions(numTests: 60, seed: BigInt.from(623)),
          (rng) => gen.sample(rng),
          (input) {
            // N over 0..40: valid iff a power of two ≥ 2.
            final n = input.$1 % 41;
            final validN = n >= 2 && (n & (n - 1)) == 0;
            try {
              scrypt([1], [2], n: n, r: 1, p: 1, dkLen: 16);
              if (!validN) return false;
            } on ScryptError catch (e) {
              if (validN || e is! ScryptInvalidN) return false;
            }
            // r/p/dkLen zero or negative reject.
            final bad = -(input.$2 % 5);
            for (final call in [
              () => scrypt([1], [2], n: 2, r: bad, p: 1, dkLen: 16),
              () => scrypt([1], [2], n: 2, r: 1, p: bad, dkLen: 16),
              () => scrypt([1], [2], n: 2, r: 1, p: 1, dkLen: bad),
            ]) {
              try {
                call();
                return false;
              } on ScryptError {
                continue;
              }
            }
            return true;
          },
        ),
      );
    });

    test('hex shape is exact', () {
      final gen = Gen.tuple3(Gen.bytes(0, 24), Gen.bytes(0, 24), Gen.u8());
      _check(
        'hex shape',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(624),
          PbtOptions(numTests: 30, seed: BigInt.from(624)),
          (rng) => gen.sample(rng),
          (input) {
            final dkLen = 1 + (input.$3 % 64);
            final hex =
                scryptHex(input.$1, input.$2, n: 8, r: 1, p: 1, dkLen: dkLen);
            if (hex.length != dkLen * 2) return false;
            return RegExp(r'^[0-9a-f]+$').hasMatch(hex);
          },
        ),
      );
    });
  });
}
