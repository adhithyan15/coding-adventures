// Unit tests for the Argon2d lane: all 16 reference tests mirrored
// case-for-case (RFC vector, hex agreement, validation matrix,
// determinism, binding, tag lengths, multi-lane, multi-pass), plus
// lane extras (empty password, error values, exact messages).
library;

import 'dart:collection';
import 'dart:typed_data';

import 'package:coding_adventures_argon2d/argon2d.dart';
import 'package:test/test.dart';

Uint8List _filled(int n, int b) => Uint8List.fromList(List<int>.filled(n, b));

/// A list reporting 2^32 elements without allocating them. Any element
/// access throws: validation must fail on the length first.
class _HugeList extends ListBase<int> {
  @override
  int get length => 0x100000000;

  @override
  set length(int _) => throw StateError('must not be resized');

  @override
  int operator [](int i) => throw StateError('must not be read');

  @override
  void operator []=(int i, int v) => throw StateError('must not be written');
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('argon2d', () {
    test('rfc9106 vector 5.1', () {
      final tag = argon2d(
        _filled(32, 0x01),
        _filled(16, 0x02),
        timeCost: 3,
        memoryCost: 32,
        parallelism: 4,
        tagLength: 32,
        opts: Argon2Options(
          key: _filled(8, 0x03),
          associatedData: _filled(12, 0x04),
        ),
      );
      expect(
        _hex(tag),
        equals(
          '512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb',
        ),
      );
    });

    test('hex matches bytes', () {
      final tag = argon2d(
        'password'.codeUnits,
        'saltsalt'.codeUnits,
        timeCost: 1,
        memoryCost: 8,
        parallelism: 1,
        tagLength: 32,
      );
      final h = argon2dHex(
        'password'.codeUnits,
        'saltsalt'.codeUnits,
        timeCost: 1,
        memoryCost: 8,
        parallelism: 1,
        tagLength: 32,
      );
      expect(h, equals(_hex(tag)));
    });

    test('short salt rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'short'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
        ),
        throwsA(equals(const SaltTooShort(5))),
      );
    });

    test('tag length too small rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 3,
        ),
        throwsA(equals(const TagLengthTooSmall(3))),
      );
    });

    test('tag length above 32 bits rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 0x100000000,
        ),
        throwsA(equals(const TagLengthTooLarge(0x100000000))),
      );
    });

    test('memory below minimum rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 1,
          parallelism: 1,
          tagLength: 32,
        ),
        throwsA(equals(const MemoryTooSmall(got: 1, min: 8))),
      );
    });

    test('zero time cost rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 0,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
        ),
        throwsA(equals(const TimeCostZero())),
      );
    });

    test('zero parallelism rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 0,
          tagLength: 32,
        ),
        throwsA(equals(const InvalidParallelism(0))),
      );
    });

    test('unsupported version rejected', () {
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
          opts: const Argon2Options(version: 0x10),
        ),
        throwsA(equals(const UnsupportedVersion(0x10))),
      );
    });

    test('deterministic', () {
      Uint8List run() => argon2d(
            'password'.codeUnits,
            'somesalt'.codeUnits,
            timeCost: 1,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
          );
      final a = run();
      final b = run();
      expect(_hex(a), equals(_hex(b)));
      expect(a.length, equals(32));
    });

    test('different passwords differ', () {
      Uint8List run(String pw) => argon2d(
            pw.codeUnits,
            'somesalt'.codeUnits,
            timeCost: 1,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
          );
      expect(_hex(run('password1')), isNot(equals(_hex(run('password2')))));
    });

    test('different salts differ', () {
      Uint8List run(String salt) => argon2d(
            'password'.codeUnits,
            salt.codeUnits,
            timeCost: 1,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
          );
      expect(_hex(run('saltsalt')), isNot(equals(_hex(run('saltsal2')))));
    });

    test('key binds', () {
      Uint8List run(Argon2Options? opts) => argon2d(
            'password'.codeUnits,
            'saltsalt'.codeUnits,
            timeCost: 1,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
            opts: opts,
          );
      final a = run(null);
      final b =
          run(Argon2Options(key: Uint8List.fromList('secret!!'.codeUnits)));
      expect(_hex(a), isNot(equals(_hex(b))));
    });

    test('associated data binds', () {
      Uint8List run(Argon2Options? opts) => argon2d(
            'password'.codeUnits,
            'saltsalt'.codeUnits,
            timeCost: 1,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
            opts: opts,
          );
      final a = run(null);
      final b = run(
          Argon2Options(associatedData: Uint8List.fromList('ad'.codeUnits)));
      expect(_hex(a), isNot(equals(_hex(b))));
    });

    test('tag length variants', () {
      for (final t in [4, 16, 32, 64, 65, 128]) {
        final tag = argon2d(
          'password'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: t,
        );
        expect(tag.length, equals(t), reason: 'tagLength $t');
      }
    });

    test('multi-lane parameters', () {
      final tag = argon2d(
        _filled(32, 0x01),
        _filled(16, 0x02),
        timeCost: 3,
        memoryCost: 32,
        parallelism: 4,
        tagLength: 32,
      );
      expect(tag.length, equals(32));
    });

    test('multiple passes', () {
      Uint8List run(int t) => argon2d(
            'password'.codeUnits,
            'saltsalt'.codeUnits,
            timeCost: t,
            memoryCost: 8,
            parallelism: 1,
            tagLength: 32,
          );
      final t1 = _hex(run(1));
      final t2 = _hex(run(2));
      final t3 = _hex(run(3));
      expect(t1, isNot(equals(t2)));
      expect(t2, isNot(equals(t3)));
      expect(t1, isNot(equals(t3)));
    });

    // --- Lane extras ---

    test('empty password hashes', () {
      final tag = argon2d(
        [],
        'saltsalt'.codeUnits,
        timeCost: 1,
        memoryCost: 8,
        parallelism: 1,
        tagLength: 32,
      );
      expect(tag.length, equals(32));
    });

    test('exact error messages', () {
      expect(
        const PasswordTooLong(5).toString(),
        equals('password length must fit in 32 bits, got 5'),
      );
      expect(
        const SaltTooShort(3).toString(),
        equals('salt must be at least 8 bytes, got 3'),
      );
      expect(
        const SaltTooLong(9).toString(),
        equals('salt length must fit in 32 bits, got 9'),
      );
      expect(
        const KeyTooLong(7).toString(),
        equals('key length must fit in 32 bits, got 7'),
      );
      expect(
        const AssociatedDataTooLong(6).toString(),
        equals('associated data length must fit in 32 bits, got 6'),
      );
      expect(
        const TagLengthTooSmall(2).toString(),
        equals('tagLength must be >= 4, got 2'),
      );
      expect(
        const TagLengthTooLarge(9).toString(),
        equals('tagLength must fit in 32 bits, got 9'),
      );
      expect(
        const InvalidParallelism(0).toString(),
        equals('parallelism must be in [1, 2^24-1], got 0'),
      );
      expect(
        const MemoryTooSmall(got: 1, min: 8).toString(),
        equals('memoryCost must be >= 8*parallelism (8), got 1'),
      );
      expect(
        const TimeCostZero().toString(),
        equals('timeCost must be >= 1'),
      );
      expect(
        const UnsupportedVersion(0x10).toString(),
        equals('only Argon2 v1.3 (0x13) is supported; got 0x10'),
      );
      expect(
        const Argon2Blake2bError('x').toString(),
        equals('blake2b error: x'),
      );
    });

    test('error values compare by type and payload', () {
      final errors = [
        const PasswordTooLong(9),
        const SaltTooShort(3),
        const SaltTooLong(9),
        const KeyTooLong(7),
        const AssociatedDataTooLong(6),
        const TagLengthTooSmall(2),
        const TagLengthTooLarge(9),
        const InvalidParallelism(0),
        const MemoryTooSmall(got: 1, min: 8),
        const TimeCostZero(),
        const UnsupportedVersion(0x10),
        const Argon2Blake2bError('x'),
      ];
      // Every variant equals itself (covering ==) and hashes stably.
      for (final error in errors) {
        expect(error == error, isTrue);
        expect(error.hashCode, equals(error.hashCode));
        expect(error.toString(), isNotEmpty);
      }
      // Distinct variants never compare equal.
      for (var i = 0; i < errors.length; i++) {
        for (var j = 0; j < errors.length; j++) {
          if (i != j) {
            expect(errors[i] == errors[j], isFalse, reason: '$i vs $j');
          }
        }
      }
      // Same variant, different payloads.
      expect(
        const SaltTooShort(3) == const SaltTooShort(4),
        isFalse,
      );
      expect(
        const MemoryTooSmall(got: 1, min: 8) ==
            const MemoryTooSmall(got: 2, min: 8),
        isFalse,
      );
    });

    test('version constant is 0x13', () {
      expect(argon2Version13, equals(0x13));
    });

    test('over-32-bit lengths fail without reading input', () {
      // A fake list reporting 2^32 elements exercises the length caps
      // without allocating gigabytes; validation must throw before
      // touching any element.
      final huge = _HugeList();
      expect(
        () => argon2d(
          huge,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
        ),
        throwsA(isA<PasswordTooLong>()),
      );
      expect(
        () => argon2d(
          'pw'.codeUnits,
          huge,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
        ),
        throwsA(isA<SaltTooLong>()),
      );
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
          opts: Argon2Options(key: huge),
        ),
        throwsA(isA<KeyTooLong>()),
      );
      expect(
        () => argon2d(
          'pw'.codeUnits,
          'saltsalt'.codeUnits,
          timeCost: 1,
          memoryCost: 8,
          parallelism: 1,
          tagLength: 32,
          opts: Argon2Options(associatedData: huge),
        ),
        throwsA(isA<AssociatedDataTooLong>()),
      );
    });
  });
}
