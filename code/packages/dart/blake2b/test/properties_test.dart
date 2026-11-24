/// Property tests for `coding_adventures_blake2b`, using the in-lane
/// `pbt` package: chunking equivalence, digest idempotence, copy
/// independence, parameter sensitivity, option validation, and
/// empty/null equivalence — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

Gen<Uint8List> _bytes(int minLen, int maxLen) => Gen.bytes(minLen, maxLen);

Gen<Blake2bOptions> _options() => Gen.tuple2(
      Gen.integer(BigInt.from(1), BigInt.from(64)),
      Gen.u8(),
    ).map((input) {
      final size = input.$1.toInt();
      final tag = input.$2;
      Uint8List? key;
      Uint8List? salt;
      Uint8List? personal;
      if (tag % 2 == 0) {
        key = Uint8List.fromList([tag, tag + 1, tag + 2]);
      }
      if (tag % 3 == 0) {
        salt = Uint8List.fromList(List<int>.filled(16, tag));
      }
      if (tag % 5 == 0) {
        personal = Uint8List.fromList(List<int>.filled(16, 255 - tag));
      }
      return Blake2bOptions(
        digestSize: size,
        key: key,
        salt: salt,
        personal: personal,
      );
    });

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

void main() {
  group('properties', () {
    test('chunking never changes the digest', () {
      final gen = Gen.tuple3(_bytes(0, 400), _options(), Gen.u8());
      final verdict = checkProperty<(Uint8List, Blake2bOptions, int)>(
        BigInt.from(401),
        PbtOptions(numTests: 200, seed: BigInt.from(401)),
        (rng) => gen.sample(rng),
        (input) {
          final data = input.$1;
          final options = input.$2;
          final want = blake2b(data, options);
          // Two random cut points -> three chunks.
          final cuts = [0, data.length];
          if (data.length > 2) {
            cuts.add(input.$3 % data.length);
            cuts.add((input.$3 * 7 + 1) % data.length);
          }
          cuts.sort();
          final h = Blake2bHasher(options);
          for (var i = 0; i + 1 < cuts.length; i++) {
            h.update(data.sublist(cuts[i], cuts[i + 1]));
          }
          return _equalBytes(h.digest(), want);
        },
      );
      _check('chunking', verdict);
    });

    test('digest is idempotent and copy is independent', () {
      final gen = Gen.tuple2(_bytes(0, 300), _options());
      final verdict = checkProperty<(Uint8List, Blake2bOptions)>(
        BigInt.from(402),
        PbtOptions(numTests: 200, seed: BigInt.from(402)),
        (rng) => gen.sample(rng),
        (input) {
          final data = input.$1;
          final options = input.$2;
          final h = Blake2bHasher(options);
          final mid = data.length ~/ 2;
          h.update(data.sublist(0, mid));
          final clone = h.copy();
          h.update(data.sublist(mid));
          clone.update(data.sublist(mid));
          // Same stream after the fork: identical digests.
          if (!_equalBytes(h.digest(), clone.digest())) return false;
          // Idempotent repeated digests.
          if (!_equalBytes(h.digest(), h.digest())) return false;
          // Diverging the clone changes only the clone.
          final before = h.digest();
          clone.update([0]);
          if (!_equalBytes(h.digest(), before)) return false;
          return true;
        },
      );
      _check('copy-idempotent', verdict);
    });

    test('parameters avalanche the digest', () {
      final gen = Gen.tuple2(_bytes(1, 200), Gen.u8());
      final verdict = checkProperty<(Uint8List, int)>(
        BigInt.from(403),
        PbtOptions(numTests: 200, seed: BigInt.from(403)),
        (rng) => gen.sample(rng),
        (input) {
          final data = input.$1;
          final base = blake2b(data);
          // One flipped input bit changes the digest.
          final flipped = Uint8List.fromList(data);
          final at = input.$2 % flipped.length;
          flipped[at] ^= 1;
          if (_equalBytes(blake2b(flipped), base)) return false;
          // Any parameter change changes the digest.
          final variants = [
            const Blake2bOptions(digestSize: 63),
            Blake2bOptions(key: Uint8List.fromList([1])),
            Blake2bOptions(salt: Uint8List.fromList(List<int>.filled(16, 7))),
            Blake2bOptions(
                personal: Uint8List.fromList(List<int>.filled(16, 9))),
          ];
          for (final options in variants) {
            final got = blake2b(data, options);
            // Truncated digests are prefixes of... no: digest_size
            // changes the parameter block, so even the 63-byte digest
            // is NOT a prefix of the 64-byte one.
            if (options.digestSize == 63) {
              if (got.length != 63) return false;
              if (_equalBytes(got, base.sublist(0, 63))) return false;
            } else if (_equalBytes(got, base)) {
              return false;
            }
          }
          return true;
        },
      );
      _check('avalanche', verdict);
    });

    test('option boundaries validate exactly', () {
      final gen = Gen.integer(BigInt.zero, BigInt.from(130));
      final verdict = checkProperty<BigInt>(
        BigInt.from(404),
        PbtOptions(numTests: 200, seed: BigInt.from(404)),
        (rng) => gen.sample(rng),
        (input) {
          final n = input.toInt();
          // Digest size: 1..64.
          var threw = false;
          try {
            Blake2bHasher(Blake2bOptions(digestSize: n));
          } on InvalidDigestSize catch (e) {
            threw = true;
            if (e.size != n) return false;
          }
          if (threw != (n < 1 || n > 64)) return false;
          // Key: 0..64.
          threw = false;
          try {
            Blake2bHasher(Blake2bOptions(key: Uint8List(n)));
          } on KeyTooLong catch (e) {
            threw = true;
            if (e.length != n) return false;
          }
          if (threw != (n > 64)) return false;
          // Salt/personal: empty or 16.
          for (final make in [
            (int m) => Blake2bOptions(salt: Uint8List(m)),
            (int m) => Blake2bOptions(personal: Uint8List(m)),
          ]) {
            threw = false;
            try {
              Blake2bHasher(make(n));
            } on Blake2bError {
              threw = true;
            }
            if (threw != (n != 0 && n != 16)) return false;
          }
          return true;
        },
      );
      _check('option-bounds', verdict);
    });

    test('empty and absent options hash identically', () {
      final gen = _bytes(0, 200);
      final verdict = checkProperty<Uint8List>(
        BigInt.from(405),
        PbtOptions(numTests: 200, seed: BigInt.from(405)),
        (rng) => gen.sample(rng),
        (data) {
          final a = blake2b(data, const Blake2bOptions());
          final b = blake2b(
            data,
            Blake2bOptions(
              key: Uint8List(0),
              salt: Uint8List(0),
              personal: Uint8List(0),
            ),
          );
          return _equalBytes(a, b);
        },
      );
      _check('empty-is-absent', verdict);
    });

    test('digest length always equals digestSize', () {
      final gen = Gen.tuple2(_bytes(0, 300), _options());
      final verdict = checkProperty<(Uint8List, Blake2bOptions)>(
        BigInt.from(406),
        PbtOptions(numTests: 200, seed: BigInt.from(406)),
        (rng) => gen.sample(rng),
        (input) {
          final options = input.$2;
          final want = options.digestSize ?? 64;
          final hasher = Blake2bHasher(options)..update(input.$1);
          if (hasher.digest().length != want) return false;
          if (hasher.hexDigest().length != want * 2) return false;
          return blake2b(input.$1, options).length == want;
        },
      );
      _check('digest-length', verdict);
    });
  });
}
