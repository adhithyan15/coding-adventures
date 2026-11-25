/// Property tests for `coding_adventures_aes`, using the in-lane
/// `pbt` package: encrypt/decrypt round-trips on every key size,
/// fixed-vs-compat agreement, determinism, avalanche, key/plaintext
/// sensitivity, and the error matrix — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart';
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

int _popCount(int x) {
  var n = 0;
  while (x != 0) {
    n += x & 1;
    x >>= 1;
  }
  return n;
}

void main() {
  group('properties', () {
    test('round trip on every key size', () {
      final gen = Gen.tuple3(Gen.bytes(16, 16), Gen.bytes(32, 32), Gen.u8());
      _check(
        'round trip',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(601),
          PbtOptions(numTests: 100, seed: BigInt.from(601)),
          (rng) => gen.sample(rng),
          (input) {
            for (final keyLen in [16, 24, 32]) {
              final key = input.$2.sublist(0, keyLen);
              final ct = encryptBlock(input.$1, key);
              if (!_equalBytes(decryptBlock(ct, key), input.$1)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('fixed agrees with compat on random inputs', () {
      final gen = Gen.tuple2(Gen.bytes(16, 16), Gen.bytes(16, 16));
      _check(
        'fixed agreement',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(602),
          PbtOptions(numTests: 100, seed: BigInt.from(602)),
          (rng) => gen.sample(rng),
          (input) {
            final fixed = Uint8List(16);
            aes128EncryptBlockFixed(input.$1, input.$2, fixed);
            return _equalBytes(fixed, encryptBlock(input.$1, input.$2));
          },
        ),
      );
    });

    test('determinism and avalanche', () {
      final gen = Gen.tuple3(Gen.bytes(16, 16), Gen.bytes(16, 16), Gen.u8());
      _check(
        'avalanche',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(603),
          PbtOptions(numTests: 60, seed: BigInt.from(603)),
          (rng) => gen.sample(rng),
          (input) {
            final a = encryptBlock(input.$1, input.$2);
            if (!_equalBytes(a, encryptBlock(input.$1, input.$2))) {
              return false;
            }
            final flipped = Uint8List.fromList(input.$1);
            flipped[input.$3 % 16] ^= 1 << (input.$3 % 8);
            final b = encryptBlock(flipped, input.$2);
            var diffBits = 0;
            for (var i = 0; i < 16; i++) {
              diffBits += _popCount(a[i] ^ b[i]);
            }
            // One flipped bit must avalanche through the rounds;
            // far more than a quarter of the bits must differ.
            return diffBits > 32;
          },
        ),
      );
    });

    test('key and plaintext sensitivity', () {
      final gen = Gen.tuple3(Gen.bytes(16, 16), Gen.bytes(16, 16), Gen.u8());
      _check(
        'sensitivity',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(604),
          PbtOptions(numTests: 60, seed: BigInt.from(604)),
          (rng) => gen.sample(rng),
          (input) {
            Uint8List flip(Uint8List v) {
              final out = Uint8List.fromList(v);
              out[input.$3 % 16] ^= 1 << (input.$3 % 8);
              return out;
            }

            final base = encryptBlock(input.$1, input.$2);
            if (_equalBytes(base, encryptBlock(flip(input.$1), input.$2))) {
              return false;
            }
            if (_equalBytes(base, encryptBlock(input.$1, flip(input.$2)))) {
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
          BigInt.from(605),
          PbtOptions(numTests: 60, seed: BigInt.from(605)),
          (rng) => gen.sample(rng),
          (input) {
            final block = Uint8List(16);
            // Key lengths 0..40: valid iff 16/24/32.
            final keyLen = input.$1 % 41;
            final key = Uint8List(keyLen);
            final validKey = keyLen == 16 || keyLen == 24 || keyLen == 32;
            try {
              encryptBlock(block, key);
              if (!validKey) return false;
            } on AesInvalidKeyLength catch (e) {
              if (validKey || e.length != keyLen) return false;
            }
            // Block lengths 0..32: valid iff exactly 16.
            final blockLen = input.$2 % 33;
            try {
              encryptBlock(Uint8List(blockLen), Uint8List(16));
              if (blockLen != 16) return false;
            } on ArgumentError {
              if (blockLen == 16) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
