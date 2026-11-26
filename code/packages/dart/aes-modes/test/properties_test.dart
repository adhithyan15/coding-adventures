/// Property tests for `coding_adventures_aes_modes`, using the
/// in-lane `pbt` package: round-trips on every mode and key size,
/// fixed-vs-compat GCM agreement, padding round-trips, tamper
/// rejection, IV/nonce sensitivity, and the error matrix — all with
/// fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes_modes/aes_modes.dart';
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
    test('padding round-trips every length', () {
      final gen = Gen.tuple2(Gen.bytes(0, 100), Gen.u8());
      _check(
        'padding',
        checkProperty<(Uint8List, int)>(
          BigInt.from(641),
          PbtOptions(numTests: 200, seed: BigInt.from(641)),
          (rng) => gen.sample(rng),
          (input) {
            final padded = pkcs7Pad(input.$1);
            if (padded.length % 16 != 0 ||
                padded.length < input.$1.length + 1 ||
                padded.length > input.$1.length + 16) {
              return false;
            }
            return _equalBytes(pkcs7Unpad(padded), input.$1);
          },
        ),
      );
    });

    test('round trip on every mode and key size', () {
      final gen = Gen.tuple3(Gen.bytes(0, 96), Gen.bytes(32, 32), Gen.u8());
      _check(
        'round trip',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(642),
          PbtOptions(numTests: 60, seed: BigInt.from(642)),
          (rng) => gen.sample(rng),
          (input) {
            final pt = input.$1;
            final iv = input.$2.sublist(0, 16);
            final nonce = input.$2.sublist(0, 12);
            final aad = input.$2.sublist(0, input.$3 % 33);
            for (final keyLen in [16, 24, 32]) {
              final key = Uint8List.fromList(
                  List<int>.generate(keyLen, (i) => input.$2[i % 32]));
              if (!_equalBytes(ecbDecrypt(ecbEncrypt(pt, key), key), pt)) {
                return false;
              }
              if (!_equalBytes(
                  cbcDecrypt(cbcEncrypt(pt, key, iv), key, iv), pt)) {
                return false;
              }
              if (!_equalBytes(
                  ctrDecrypt(ctrEncrypt(pt, key, nonce), key, nonce), pt)) {
                return false;
              }
              final (ct, tag) = gcmEncrypt(pt, key, nonce, aad);
              if (tag.length != 16) return false;
              if (!_equalBytes(gcmDecrypt(ct, key, nonce, aad, tag), pt)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('fixed GCM agrees with compat', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 96),
        Gen.bytes(0, 48),
        Gen.tuple2(Gen.bytes(16, 16), Gen.bytes(12, 12)),
      );
      _check(
        'fixed agreement',
        checkProperty<(Uint8List, Uint8List, (Uint8List, Uint8List))>(
          BigInt.from(643),
          PbtOptions(numTests: 60, seed: BigInt.from(643)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$3.$1;
            final nonce = input.$3.$2;
            final fixedCt = Uint8List(input.$1.length);
            final fixedTag = Uint8List(16);
            aes128GcmEncryptFixed(
                input.$1, key, nonce, input.$2, fixedCt, fixedTag);
            final (ct, tag) = gcmEncrypt(input.$1, key, nonce, input.$2);
            if (!_equalBytes(fixedCt, ct) || !_equalBytes(fixedTag, tag)) {
              return false;
            }
            final opened = Uint8List(ct.length);
            aes128GcmDecryptFixed(ct, key, nonce, input.$2, tag, opened);
            return _equalBytes(opened, input.$1);
          },
        ),
      );
    });

    test('tampering breaks authentication', () {
      final gen = Gen.tuple3(Gen.bytes(1, 64), Gen.bytes(16, 16), Gen.u8());
      _check(
        'tamper rejection',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(644),
          PbtOptions(numTests: 60, seed: BigInt.from(644)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$2;
            final nonce = input.$2.sublist(0, 12);
            final aad = input.$2.sublist(0, input.$3 % 17);
            final (ct, tag) = gcmEncrypt(input.$1, key, nonce, aad);
            if (ct.isEmpty) return true;
            final where = input.$3 % 3;
            final ct2 = Uint8List.fromList(ct);
            final aad2 = Uint8List.fromList(aad);
            final tag2 = Uint8List.fromList(tag);
            if (where == 0) {
              ct2[input.$3 % ct2.length] ^= 1;
            } else if (where == 1 && aad2.isNotEmpty) {
              aad2[input.$3 % aad2.length] ^= 1;
            } else {
              tag2[input.$3 % 16] ^= 1;
            }
            try {
              gcmDecrypt(ct2, key, nonce, aad2, tag2);
              return false;
            } on GcmAuthenticationFailed {
              return true;
            }
          },
        ),
      );
    });

    test('IV and nonce sensitivity', () {
      final gen = Gen.tuple2(Gen.bytes(1, 48), Gen.bytes(32, 32));
      _check(
        'iv sensitivity',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(645),
          PbtOptions(numTests: 60, seed: BigInt.from(645)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$2.sublist(0, 16);
            final iv = Uint8List.fromList(input.$2.sublist(0, 16));
            final iv2 = Uint8List.fromList(iv)..[0] ^= 1;
            if (_equalBytes(cbcEncrypt(input.$1, key, iv),
                cbcEncrypt(input.$1, key, iv2))) {
              return false;
            }
            final nonce = Uint8List.fromList(input.$2.sublist(0, 12));
            final nonce2 = Uint8List.fromList(nonce)..[0] ^= 1;
            return !_equalBytes(ctrEncrypt(input.$1, key, nonce),
                ctrEncrypt(input.$1, key, nonce2));
          },
        ),
      );
    });

    test('validation rejects exactly the bad shapes', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'validation matrix',
        checkProperty<(int, int)>(
          BigInt.from(646),
          PbtOptions(numTests: 100, seed: BigInt.from(646)),
          (rng) => gen.sample(rng),
          (input) {
            final key = Uint8List(16);
            // IV/nonce/tag widths: valid iff exact.
            final ivLen = input.$1 % 20;
            try {
              cbcEncrypt(Uint8List(0), key, Uint8List(ivLen));
              if (ivLen != 16) return false;
            } on CbcInvalidIv catch (e) {
              if (ivLen == 16 || e.length != ivLen) return false;
            }
            final nonceLen = input.$2 % 16;
            try {
              ctrEncrypt(Uint8List(0), key, Uint8List(nonceLen));
              if (nonceLen != 12) return false;
            } on CtrInvalidNonce catch (e) {
              if (nonceLen == 12 || e.length != nonceLen) return false;
            }
            // Ciphertext widths: valid iff non-empty multiple of 16.
            final ctLen = (input.$1 + input.$2) % 40;
            final validCt = ctLen != 0 && ctLen % 16 == 0;
            try {
              ecbDecrypt(Uint8List(ctLen), key);
              // May still throw InvalidPadding after structural
              // acceptance; only the structural verdict matters here.
              if (!validCt) return false;
            } on EcbInvalidCiphertext {
              if (validCt) return false;
            } on InvalidPadding {
              if (!validCt) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
