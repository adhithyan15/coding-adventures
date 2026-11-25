/// Property tests for `coding_adventures_chacha20_poly1305`, using
/// the in-lane `pbt` package: stream and AEAD round-trips, fixed
/// agreement, determinism, tamper rejection, nonce/key sensitivity,
/// XChaCha20 round-trips, and the validation matrix — all with
/// fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_chacha20_poly1305/chacha20_poly1305.dart';
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

void main() {
  group('properties', () {
    test('stream round trip and determinism', () {
      final gen = Gen.tuple3(Gen.bytes(0, 200), Gen.bytes(32, 32), Gen.u8());
      _check(
        'stream round trip',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(661),
          PbtOptions(numTests: 60, seed: BigInt.from(661)),
          (rng) => gen.sample(rng),
          (input) {
            final nonce = input.$2.sublist(0, 12);
            final counter = input.$3 % 100;
            final ct = chacha20Encrypt(input.$1, input.$2, nonce, counter);
            if (ct.length != input.$1.length) return false;
            if (!_equalBytes(
                chacha20Encrypt(ct, input.$2, nonce, counter), input.$1)) {
              return false;
            }
            return _equalBytes(
                chacha20Encrypt(input.$1, input.$2, nonce, counter), ct);
          },
        ),
      );
    });

    test('AEAD round trip and tamper rejection', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 128),
        Gen.bytes(0, 48),
        Gen.tuple2(Gen.bytes(32, 32), Gen.bytes(12, 12)),
      );
      _check(
        'aead round trip',
        checkProperty<(Uint8List, Uint8List, (Uint8List, Uint8List))>(
          BigInt.from(662),
          PbtOptions(numTests: 60, seed: BigInt.from(662)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$3.$1;
            final nonce = input.$3.$2;
            final (ct, tag) = aeadEncrypt(input.$1, key, nonce, input.$2);
            if (!_equalBytes(
                aeadDecrypt(ct, key, nonce, input.$2, tag)!, input.$1)) {
              return false;
            }
            if (ct.isEmpty) return true;
            final tampered = Uint8List.fromList(ct);
            tampered[tampered.length ~/ 2] ^= 1;
            return aeadDecrypt(tampered, key, nonce, input.$2, tag) == null;
          },
        ),
      );
    });

    test('fixed agrees with compat everywhere', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 200),
        Gen.bytes(0, 48),
        Gen.tuple2(Gen.bytes(32, 32), Gen.bytes(12, 12)),
      );
      _check(
        'fixed agreement',
        checkProperty<(Uint8List, Uint8List, (Uint8List, Uint8List))>(
          BigInt.from(663),
          PbtOptions(numTests: 60, seed: BigInt.from(663)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$3.$1;
            final nonce = input.$3.$2;
            // Stream.
            final fixedStream = Uint8List(input.$1.length);
            chacha20XorFixed(input.$1, key, nonce, 3, fixedStream);
            if (!_equalBytes(
                fixedStream, chacha20Encrypt(input.$1, key, nonce, 3))) {
              return false;
            }
            // MAC.
            final fixedMac = Uint8List(16);
            poly1305MacFixed(input.$1, key, fixedMac);
            if (!_equalBytes(fixedMac, poly1305Mac(input.$1, key))) {
              return false;
            }
            // AEAD.
            final fixedCt = Uint8List(input.$1.length);
            final fixedTag = Uint8List(16);
            aeadEncryptFixed(input.$1, key, nonce, input.$2, fixedCt, fixedTag);
            final (ct, tag) = aeadEncrypt(input.$1, key, nonce, input.$2);
            return _equalBytes(fixedCt, ct) && _equalBytes(fixedTag, tag);
          },
        ),
      );
    });

    test('nonce and key sensitivity', () {
      final gen = Gen.tuple3(Gen.bytes(1, 64), Gen.bytes(32, 32), Gen.u8());
      _check(
        'sensitivity',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(664),
          PbtOptions(numTests: 60, seed: BigInt.from(664)),
          (rng) => gen.sample(rng),
          (input) {
            final nonce = input.$2.sublist(0, 12);
            final base = chacha20Encrypt(input.$1, input.$2, nonce, 0);
            final nonce2 = Uint8List.fromList(nonce);
            nonce2[input.$3 % 12] ^= 1;
            if (_equalBytes(
                base, chacha20Encrypt(input.$1, input.$2, nonce2, 0))) {
              return false;
            }
            final key2 = Uint8List.fromList(input.$2);
            key2[input.$3 % 32] ^= 1;
            return !_equalBytes(
                base, chacha20Encrypt(input.$1, key2, nonce, 0));
          },
        ),
      );
    });

    test('XChaCha20 round trip', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 128),
        Gen.bytes(0, 32),
        Gen.tuple2(Gen.bytes(32, 32), Gen.bytes(24, 24)),
      );
      _check(
        'xchacha round trip',
        checkProperty<(Uint8List, Uint8List, (Uint8List, Uint8List))>(
          BigInt.from(665),
          PbtOptions(numTests: 40, seed: BigInt.from(665)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$3.$1;
            final nonce = input.$3.$2;
            final ct = xchacha20Encrypt(input.$1, key, nonce, 1);
            if (!_equalBytes(xchacha20Encrypt(ct, key, nonce, 1), input.$1)) {
              return false;
            }
            final (act, tag) =
                xchacha20Poly1305AeadEncrypt(input.$1, key, nonce, input.$2);
            final opened =
                xchacha20Poly1305AeadDecrypt(act, key, nonce, input.$2, tag);
            return opened != null && _equalBytes(opened, input.$1);
          },
        ),
      );
    });

    test('validation rejects exactly the bad shapes', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'validation matrix',
        checkProperty<(int, int)>(
          BigInt.from(666),
          PbtOptions(numTests: 100, seed: BigInt.from(666)),
          (rng) => gen.sample(rng),
          (input) {
            final keyLen = input.$1 % 40;
            final validKey = keyLen == 32;
            try {
              chacha20Encrypt(
                  Uint8List(0), Uint8List(keyLen), Uint8List(12), 0);
              if (!validKey) return false;
            } on ArgumentError {
              if (validKey) return false;
            }
            final nonceLen = input.$2 % 16;
            try {
              chacha20Encrypt(
                  Uint8List(0), Uint8List(32), Uint8List(nonceLen), 0);
              if (nonceLen != 12) return false;
            } on ArgumentError {
              if (nonceLen == 12) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
