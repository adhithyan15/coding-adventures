/// Property tests for `coding_adventures_hmac`, using the in-lane
/// `pbt` package: exact tag lengths, determinism, key/message
/// sensitivity, generic-vs-fixed agreement, segmented equivalence,
/// verify symmetry, hex shape, and empty-key rejection — all with
/// fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_hmac/hmac.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256lane;
import 'package:coding_adventures_sha384/sha384.dart' as sha384lane;
import 'package:test/test.dart';

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

final _hexShape = RegExp(r'^[0-9a-f]+$');

void main() {
  group('properties', () {
    test('tag lengths are exact', () {
      final gen = Gen.tuple2(Gen.bytes(1, 200), Gen.bytes(0, 300));
      _check(
        'tag lengths',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(701),
          PbtOptions(numTests: 120, seed: BigInt.from(701)),
          (rng) => gen.sample(rng),
          (input) =>
              hmacMd5(input.$1, input.$2).length == 16 &&
              hmacSha1(input.$1, input.$2).length == 20 &&
              hmacSha256(input.$1, input.$2).length == 32 &&
              hmacSha384(input.$1, input.$2).length == 48 &&
              hmacSha512(input.$1, input.$2).length == 64,
        ),
      );
    });

    test('determinism across variants', () {
      final gen = Gen.tuple2(Gen.bytes(1, 150), Gen.bytes(0, 200));
      _check(
        'determinism',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(702),
          PbtOptions(numTests: 120, seed: BigInt.from(702)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$1;
            final msg = input.$2;
            return _equalBytes(hmacMd5(key, msg), hmacMd5(key, msg)) &&
                _equalBytes(hmacSha1(key, msg), hmacSha1(key, msg)) &&
                _equalBytes(hmacSha256(key, msg), hmacSha256(key, msg)) &&
                _equalBytes(hmacSha384(key, msg), hmacSha384(key, msg)) &&
                _equalBytes(hmacSha512(key, msg), hmacSha512(key, msg));
          },
        ),
      );
    });

    test('key sensitivity', () {
      final gen = Gen.tuple2(Gen.bytes(1, 150), Gen.bytes(0, 200));
      _check(
        'key sensitivity',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(703),
          PbtOptions(numTests: 120, seed: BigInt.from(703)),
          (rng) => gen.sample(rng),
          (input) {
            final key = Uint8List.fromList(input.$1);
            final msg = input.$2;
            key[key.length - 1] ^= 0x80;
            return !_equalBytes(
                    hmacSha256(input.$1, msg), hmacSha256(key, msg)) &&
                !_equalBytes(hmacSha384(input.$1, msg), hmacSha384(key, msg)) &&
                !_equalBytes(hmacSha512(input.$1, msg), hmacSha512(key, msg)) &&
                !_equalBytes(hmacMd5(input.$1, msg), hmacMd5(key, msg)) &&
                !_equalBytes(hmacSha1(input.$1, msg), hmacSha1(key, msg));
          },
        ),
      );
    });

    test('message sensitivity and avalanche', () {
      final gen = Gen.tuple2(Gen.bytes(1, 150), Gen.bytes(1, 200));
      _check(
        'message sensitivity',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(704),
          PbtOptions(numTests: 120, seed: BigInt.from(704)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$1;
            final msg = Uint8List.fromList(input.$2);
            msg[0] ^= 0x01;
            final a = hmacSha256(key, input.$2);
            final b = hmacSha256(key, msg);
            if (_equalBytes(a, b)) return false;
            var changed = 0;
            for (var i = 0; i < a.length; i++) {
              if (a[i] != b[i]) changed++;
            }
            // One flipped bit avalanches through both nested hashes;
            // far more than a quarter of the bytes must differ.
            if (changed < 8) return false;
            return !_equalBytes(
                    hmacSha384(key, input.$2), hmacSha384(key, msg)) &&
                !_equalBytes(hmacSha512(key, input.$2), hmacSha512(key, msg));
          },
        ),
      );
    });

    test('generic agrees with fixed-memory paths', () {
      final gen = Gen.tuple2(Gen.bytes(1, 200), Gen.bytes(0, 300));
      _check(
        'generic agreement',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(705),
          PbtOptions(numTests: 60, seed: BigInt.from(705)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$1;
            final msg = input.$2;
            return _equalBytes(hmacSha256(key, msg),
                    hmac(sha256lane.sha256, 64, key, msg)) &&
                _equalBytes(hmacSha384(key, msg),
                    hmac(sha384lane.sha384, 128, key, msg));
          },
        ),
      );
    });

    test('random splits never change the segments tag', () {
      final gen = Gen.tuple3(Gen.bytes(1, 150), Gen.bytes(0, 300), Gen.u8());
      _check(
        'segments equivalence',
        checkProperty<(Uint8List, Uint8List, int)>(
          BigInt.from(706),
          PbtOptions(numTests: 60, seed: BigInt.from(706)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$1;
            final msg = input.$2;
            final cut = msg.isEmpty ? 0 : input.$3 % (msg.length + 1);
            final segments = [
              msg.sublist(0, cut),
              <int>[],
              msg.sublist(cut),
            ];
            return _equalBytes(
                    hmacSha256Segments(key, segments), hmacSha256(key, msg)) &&
                _equalBytes(
                    hmacSha384Segments(key, segments), hmacSha384(key, msg));
          },
        ),
      );
    });

    test('verify is symmetric and self-accepting', () {
      final gen = Gen.tuple2(Gen.bytes(1, 150), Gen.bytes(0, 200));
      _check(
        'verify symmetry',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(707),
          PbtOptions(numTests: 120, seed: BigInt.from(707)),
          (rng) => gen.sample(rng),
          (input) {
            final a = hmacSha256(input.$1, input.$2);
            final b = hmacSha512(input.$1, input.$2);
            if (!verifyHmac(a, a) || !verifyHmac(b, b)) return false;
            if (verifyHmac(a, b) != verifyHmac(b, a)) return false;
            final flipped = Uint8List.fromList(a);
            flipped[flipped.length ~/ 2] ^= 0x01;
            return !verifyHmac(a, flipped) && !verifyHmac(flipped, a);
          },
        ),
      );
    });

    test('hex shape is exact', () {
      final gen = Gen.tuple2(Gen.bytes(1, 150), Gen.bytes(0, 200));
      _check(
        'hex shape',
        checkProperty<(Uint8List, Uint8List)>(
          BigInt.from(708),
          PbtOptions(numTests: 60, seed: BigInt.from(708)),
          (rng) => gen.sample(rng),
          (input) {
            final key = input.$1;
            final msg = input.$2;
            final hexes = [
              (hmacMd5Hex(key, msg), 32),
              (hmacSha1Hex(key, msg), 40),
              (hmacSha256Hex(key, msg), 64),
              (hmacSha384Hex(key, msg), 96),
              (hmacSha512Hex(key, msg), 128),
            ];
            for (final (hex, length) in hexes) {
              if (hex.length != length || !_hexShape.hasMatch(hex)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('empty key rejected on every entry point', () {
      final gen = Gen.bytes(0, 100);
      _check(
        'empty key rejection',
        checkProperty<Uint8List>(
          BigInt.from(709),
          PbtOptions(numTests: 100, seed: BigInt.from(709)),
          (rng) => gen.sample(rng),
          (msg) {
            final empty = Uint8List(0);
            final throws = [
              () => hmacMd5(empty, msg),
              () => hmacSha1(empty, msg),
              () => hmacSha256(empty, msg),
              () => hmacSha384(empty, msg),
              () => hmacSha512(empty, msg),
              () => hmacSha256Segments(empty, [msg]),
              () => hmacSha384Segments(empty, [msg]),
              () => hmacMd5Hex(empty, msg),
              () => hmacSha1Hex(empty, msg),
              () => hmacSha256Hex(empty, msg),
              () => hmacSha384Hex(empty, msg),
              () => hmacSha512Hex(empty, msg),
            ];
            for (final fn in throws) {
              try {
                fn();
              } on HmacEmptyKey {
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
