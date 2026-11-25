/// Property tests for `coding_adventures_argon2d`, using the in-lane
/// `pbt` package: determinism, input binding (password/salt/key/AD),
/// parameter binding (time/memory/lanes/tag), tag lengths, and the
/// validation matrix — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_argon2d/argon2d.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

Uint8List _hash(
  Uint8List password,
  Uint8List salt, {
  int timeCost = 1,
  int memoryCost = 8,
  int parallelism = 1,
  int tagLength = 32,
  Argon2Options? opts,
}) =>
    argon2d(password, salt,
        timeCost: timeCost,
        memoryCost: memoryCost,
        parallelism: parallelism,
        tagLength: tagLength,
        opts: opts);

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
    test('hashing is deterministic', () {
      final gen = Gen.tuple2(Gen.bytes(0, 32), Gen.bytes(8, 24));
      final verdict = checkProperty<(Uint8List, Uint8List)>(
        BigInt.from(501),
        PbtOptions(numTests: 60, seed: BigInt.from(501)),
        (rng) => gen.sample(rng),
        (input) => _equalBytes(
          _hash(input.$1, input.$2),
          _hash(input.$1, input.$2),
        ),
      );
      _check('determinism', verdict);
    });

    test('every input binds the tag', () {
      final gen = Gen.tuple3(
        Gen.bytes(1, 24),
        Gen.bytes(8, 24),
        Gen.u8(),
      );
      final verdict = checkProperty<(Uint8List, Uint8List, int)>(
        BigInt.from(502),
        PbtOptions(numTests: 60, seed: BigInt.from(502)),
        (rng) => gen.sample(rng),
        (input) {
          final password = input.$1;
          final salt = input.$2;
          final base = _hash(password, salt);
          // Flip one password bit.
          final pw2 = Uint8List.fromList(password);
          pw2[input.$3 % pw2.length] ^= 1;
          if (_equalBytes(_hash(pw2, salt), base)) return false;
          // Flip one salt bit.
          final salt2 = Uint8List.fromList(salt);
          salt2[input.$3 % salt2.length] ^= 1;
          if (_equalBytes(_hash(password, salt2), base)) return false;
          // Adding a key or AD changes the tag.
          final keyed = _hash(
            password,
            salt,
            opts: Argon2Options(key: Uint8List.fromList([7])),
          );
          if (_equalBytes(keyed, base)) return false;
          final aded = _hash(
            password,
            salt,
            opts: Argon2Options(associatedData: Uint8List.fromList([9])),
          );
          return !_equalBytes(aded, base);
        },
      );
      _check('input-binding', verdict);
    });

    test('every cost parameter binds the tag', () {
      final gen = Gen.tuple2(Gen.bytes(1, 16), Gen.bytes(8, 16));
      final verdict = checkProperty<(Uint8List, Uint8List)>(
        BigInt.from(503),
        PbtOptions(numTests: 40, seed: BigInt.from(503)),
        (rng) => gen.sample(rng),
        (input) {
          final password = input.$1;
          final salt = input.$2;
          final base = _hash(password, salt,
              timeCost: 1, memoryCost: 16, parallelism: 2, tagLength: 32);
          if (_equalBytes(
              _hash(password, salt,
                  timeCost: 2, memoryCost: 16, parallelism: 2, tagLength: 32),
              base)) {
            return false;
          }
          if (_equalBytes(
              _hash(password, salt,
                  timeCost: 1, memoryCost: 32, parallelism: 2, tagLength: 32),
              base)) {
            return false;
          }
          if (_equalBytes(
              _hash(password, salt,
                  timeCost: 1, memoryCost: 16, parallelism: 1, tagLength: 32),
              base)) {
            return false;
          }
          return true;
        },
      );
      _check('param-binding', verdict);
    });

    test('tag length truncates and extends output', () {
      final gen = Gen.tuple2(Gen.bytes(1, 16), Gen.bytes(8, 16));
      final verdict = checkProperty<(Uint8List, Uint8List)>(
        BigInt.from(504),
        PbtOptions(numTests: 40, seed: BigInt.from(504)),
        (rng) => gen.sample(rng),
        (input) {
          for (final t in [4, 16, 32, 64, 65, 128]) {
            final tag = _hash(input.$1, input.$2, tagLength: t);
            if (tag.length != t) return false;
          }
          // Different lengths are different outputs (H' construction).
          final short = _hash(input.$1, input.$2, tagLength: 16);
          final long = _hash(input.$1, input.$2, tagLength: 32);
          if (_equalBytes(short, long.sublist(0, 16))) return false;
          return true;
        },
      );
      _check('tag-lengths', verdict);
    });

    test('validation rejects exactly the bad shapes', () {
      final gen = Gen.tuple3(Gen.u8(), Gen.u8(), Gen.u8());
      final verdict = checkProperty<(int, int, int)>(
        BigInt.from(505),
        PbtOptions(numTests: 100, seed: BigInt.from(505)),
        (rng) => gen.sample(rng),
        (input) {
          String? idOf(
            Uint8List password,
            Uint8List salt, {
            int timeCost = 1,
            int memoryCost = 8,
            int parallelism = 1,
            int tagLength = 32,
            Argon2Options? opts,
          }) {
            try {
              argon2d(password, salt,
                  timeCost: timeCost,
                  memoryCost: memoryCost,
                  parallelism: parallelism,
                  tagLength: tagLength,
                  opts: opts);
              return null;
            } on Argon2Error catch (e) {
              return e.runtimeType.toString();
            }
          }

          final saltLen = input.$1 % 12;
          // Valid iff the salt has 8+ bytes.
          if ((idOf(Uint8List(4), Uint8List(saltLen)) == null) !=
              (saltLen >= 8)) {
            return false;
          }
          if (idOf(Uint8List(4), Uint8List(8), tagLength: 3) !=
              'TagLengthTooSmall') {
            return false;
          }
          if (idOf(Uint8List(4), Uint8List(8), timeCost: 0) != 'TimeCostZero') {
            return false;
          }
          if (idOf(Uint8List(4), Uint8List(8), parallelism: input.$2 % 2) !=
              (input.$2 % 2 == 0 ? 'InvalidParallelism' : null)) {
            return false;
          }
          if (idOf(Uint8List(4), Uint8List(8),
                  opts: const Argon2Options(version: 0x10)) !=
              'UnsupportedVersion') {
            return false;
          }
          return true;
        },
      );
      _check('validation-matrix', verdict);
    });
  });
}
