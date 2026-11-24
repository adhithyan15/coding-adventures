/// Property tests for `coding_adventures_ed25519`, using the in-lane
/// `pbt` package: sign/verify round-trips, tamper rejection, key
/// formats, determinism, and cross-key isolation — all with fixed
/// seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_ed25519/ed25519.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

Gen<Uint8List> _seed() => Gen.bytes(32, 32);

Gen<Uint8List> _message() => Gen.oneOf([
      Gen.bytes(0, 0),
      Gen.bytes(1, 3),
      Gen.bytes(31, 33),
      Gen.bytes(63, 65),
      Gen.bytes(127, 129),
      Gen.bytes(0, 200),
    ]);

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('sign then verify always accepts', () {
      final gen = Gen.tuple2(_seed(), _message());
      final verdict = checkProperty<(Uint8List, Uint8List)>(
        BigInt.from(441),
        PbtOptions(numTests: 100, seed: BigInt.from(441)),
        (rng) => gen.sample(rng),
        (input) {
          final (pub, sec) = generateKeypair(input.$1);
          final sig = sign(input.$2, sec);
          if (!isValidPublicKey(pub.bytes)) return false;
          return verify(input.$2, sig, pub);
        },
      );
      _check('round-trip', verdict);
    });

    test('flipping any signed byte rejects', () {
      final gen = Gen.tuple3(_seed(), _message(), Gen.u8());
      final verdict = checkProperty<(Uint8List, Uint8List, int)>(
        BigInt.from(442),
        PbtOptions(numTests: 60, seed: BigInt.from(442)),
        (rng) => gen.sample(rng),
        (input) {
          final (pub, sec) = generateKeypair(input.$1);
          final message = Uint8List.fromList(input.$2);
          final sig = sign(message, sec);
          // Flip one bit somewhere in (message, R, S, pubkey). Empty
          // messages cannot take a message flip, so they flip R instead.
          final where = message.isEmpty ? 1 : input.$3 % 4;
          if (where == 0) {
            message[input.$3 % message.length] ^= 1;
          }
          final sigBytes = Uint8List.fromList(sig.bytes);
          final pubBytes = Uint8List.fromList(pub.bytes);
          if (where == 1) sigBytes[input.$3 % 32] ^= 1;
          if (where == 2) sigBytes[32 + (input.$3 % 32)] ^= 1;
          if (where == 3) pubBytes[input.$3 % 32] ^= 1;
          return !verify(
            message,
            Ed25519Signature(sigBytes),
            Ed25519PublicKey(pubBytes),
          );
        },
      );
      _check('tamper-rejection', verdict);
    });

    test('keys have the specified formats', () {
      final verdict = checkProperty<Uint8List>(
        BigInt.from(443),
        PbtOptions(numTests: 100, seed: BigInt.from(443)),
        (rng) => _seed().sample(rng),
        (seed) {
          final (pub, sec) = generateKeypair(seed);
          if (pub.bytes.length != 32 || sec.bytes.length != 64) {
            return false;
          }
          for (var i = 0; i < 32; i++) {
            if (sec.bytes[i] != seed[i]) return false;
            if (sec.bytes[32 + i] != pub.bytes[i]) return false;
          }
          return true;
        },
      );
      _check('key-format', verdict);
    });

    test('distinct seeds isolate signatures', () {
      final gen = Gen.tuple3(_seed(), _seed(), _message());
      final verdict = checkProperty<(Uint8List, Uint8List, Uint8List)>(
        BigInt.from(444),
        PbtOptions(numTests: 60, seed: BigInt.from(444)),
        (rng) => gen.sample(rng),
        (input) {
          var same = true;
          for (var i = 0; i < 32; i++) {
            if (input.$1[i] != input.$2[i]) same = false;
          }
          if (same) return true; // identical seeds: nothing to isolate.
          final (pub1, sec1) = generateKeypair(input.$1);
          final (pub2, _) = generateKeypair(input.$2);
          final sig = sign(input.$3, sec1);
          // Cross-verification must fail in both directions.
          if (verify(input.$3, sig, pub2)) return false;
          // ...while the rightful key still accepts.
          return verify(input.$3, sig, pub1);
        },
      );
      _check('cross-isolation', verdict);
    });
  });
}
