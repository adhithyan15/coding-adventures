/// Unit tests for `coding_adventures_scrypt`: all 55 reference
/// tests mirrored case-for-case (22 unit + 33 integration,
/// including the two heavy RFC vectors the reference keeps behind
/// `#[ignore]` — measured fast enough here to run always), plus
/// lane extras (the PBKDF2-cap overflow path, exact error texts,
/// value equality). Salsa tests import `src/salsa.dart` directly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_scrypt/scrypt.dart';
import 'package:coding_adventures_scrypt/src/salsa.dart' as salsa;
import 'package:test/test.dart';

Uint8List _filled(int n, int b) => Uint8List.fromList(List<int>.filled(n, b));

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('salsa20/8', () {
    test('deterministic', () {
      final input = _filled(64, 1);
      expect(salsa.salsa208(input), equals(salsa.salsa208(input)));
    });

    test('output length', () {
      expect(salsa.salsa208(_filled(64, 0xab)).length, equals(64));
    });

    test('different inputs produce different outputs', () {
      expect(salsa.salsa208(_filled(64, 0x01)),
          isNot(equals(salsa.salsa208(_filled(64, 0x02)))));
    });

    test('nonzero for nonzero input', () {
      expect(salsa.salsa208(_filled(64, 0x42)), isNot(equals(Uint8List(64))));
    });

    test('length checked', () {
      expect(() => salsa.salsa208(Uint8List(63)), throwsArgumentError);
      expect(() => salsa.salsa208(Uint8List(65)), throwsArgumentError);
    });
  });

  group('RFC 7914 vectors', () {
    test('vector 1 empty password and salt', () {
      expect(
        scryptHex([], [], n: 16, r: 1, p: 1, dkLen: 64),
        equals('77d6576238657b203b19ca42c18a0497'
            'f16b4844e3074ae8dfdffa3fede21442'
            'fcd0069ded0948f8326a753a0fc81f17'
            'e8d3e0fb2e0d3628cf35e20c38d18906'),
      );
    });

    test('vector 2 password nacl', () {
      expect(
        scryptHex('password'.codeUnits, 'NaCl'.codeUnits,
            n: 1024, r: 8, p: 16, dkLen: 64),
        equals('fdbabe1c9d3472007856e7190d01e9fe'
            '7c6ad7cbc8237830e77376634b373162'
            '2eaf30d92e22a3886ff109279d9830da'
            'c727afb94a83ee6d8360cbdfa2cc0640'),
      );
    }, timeout: Timeout(Duration(minutes: 5)));

    test('vector 3 pleaseletmein', () {
      // NOTE: the reference's stored vector-3 string is a transcription
      // typo (its test is #[ignore]d, so CI never caught it). Proven by
      // three-way agreement on the value below: this lane,
      // CPython-hashlib, AND the reference implementation itself (its
      // ignored test fails against its own stored string; left side
      // matches this value byte-for-byte). Logged as a Rust follow-up.
      expect(
        scryptHex('pleaseletmein'.codeUnits, 'SodiumChloride'.codeUnits,
            n: 16384, r: 8, p: 1, dkLen: 64),
        equals('7023bdcb3afd7348461c06cd81fd38eb'
            'fda8fbba904f8e3ea9b543f6545da1f2'
            'd5432955613f0fcf62d49705242a9af9'
            'e61e85dc0d651e40dfcf017b45575887'),
      );
    }, timeout: Timeout(Duration(minutes: 5)));
  });

  group('hex wrapper', () {
    test('matches bytes', () {
      final bytes = scrypt('test'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final direct = scryptHex('test'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(direct, equals(_hex(bytes)));
    });

    test('length', () {
      expect(
          scryptHex('pw'.codeUnits, 'salt'.codeUnits,
                  n: 16, r: 1, p: 1, dkLen: 32)
              .length,
          equals(64));
    });

    test('lowercase', () {
      final hex = scryptHex('pw'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(
          hex.codeUnits.every(
              (c) => (c >= 0x30 && c <= 0x39) || (c >= 0x61 && c <= 0x66)),
          isTrue,
          reason: 'hex output should be lowercase: $hex');
    });
  });

  group('output length', () {
    test('one byte', () {
      expect(
          scrypt('pw'.codeUnits, 's'.codeUnits, n: 16, r: 1, p: 1, dkLen: 1)
              .length,
          equals(1));
    });

    test('32 bytes', () {
      expect(
          scrypt('pw'.codeUnits, 's'.codeUnits, n: 16, r: 1, p: 1, dkLen: 32)
              .length,
          equals(32));
    });

    test('64 bytes', () {
      expect(
          scrypt('pw'.codeUnits, 's'.codeUnits, n: 16, r: 1, p: 1, dkLen: 64)
              .length,
          equals(64));
    });

    test('not power of 32', () {
      expect(
          scrypt('pw'.codeUnits, 's'.codeUnits, n: 16, r: 1, p: 1, dkLen: 33)
              .length,
          equals(33));
    });

    test('matches dk len sweep', () {
      for (final len in [1, 16, 32, 64, 100]) {
        final result = scrypt('pw'.codeUnits, 'salt'.codeUnits,
            n: 16, r: 1, p: 1, dkLen: len);
        expect(result.length, equals(len), reason: 'dk_len=$len');
      }
    });
  });

  group('determinism and sensitivity', () {
    test('deterministic', () {
      Uint8List run() => scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(run(), equals(run()));
    });

    test('password sensitivity', () {
      final a = scrypt('password1'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final b = scrypt('password2'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(a, isNot(equals(b)));
    });

    test('salt sensitivity', () {
      final a = scrypt('password'.codeUnits, 'salt1'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final b = scrypt('password'.codeUnits, 'salt2'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(a, isNot(equals(b)));
    });

    test('n sensitivity', () {
      final a = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final b = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 32, r: 1, p: 1, dkLen: 32);
      expect(a, isNot(equals(b)));
    });

    test('r sensitivity', () {
      final a = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final b = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 2, p: 1, dkLen: 32);
      expect(a, isNot(equals(b)));
    });

    test('p sensitivity', () {
      final a = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      final b = scrypt('password'.codeUnits, 'salt'.codeUnits,
          n: 16, r: 1, p: 2, dkLen: 32);
      expect(a, isNot(equals(b)));
    });
  });

  group('empty inputs', () {
    test('empty password allowed', () {
      final result = scrypt([], 'salt'.codeUnits, n: 16, r: 1, p: 1, dkLen: 32);
      expect(result.length, equals(32));
    });

    test('empty salt allowed', () {
      final result =
          scrypt('password'.codeUnits, [], n: 16, r: 1, p: 1, dkLen: 32);
      expect(result.length, equals(32));
    });

    test('both empty allowed', () {
      final result = scrypt([], [], n: 16, r: 1, p: 1, dkLen: 32);
      expect(result.length, equals(32));
    });

    test('large password allowed', () {
      final result = scrypt(_filled(1000, 0x42), 'salt'.codeUnits,
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(result.length, equals(32));
    });

    test('large salt allowed', () {
      final result = scrypt('pw'.codeUnits, _filled(500, 0x99),
          n: 16, r: 1, p: 1, dkLen: 32);
      expect(result.length, equals(32));
    });
  });

  group('validation', () {
    test('n less than 2', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 1, r: 1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptInvalidN())),
      );
    });

    test('n zero', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 0, r: 1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptInvalidN())),
      );
    });

    test('n not power of two', () {
      for (final n in [3, 5, 6, 12, 100]) {
        expect(
          () =>
              scrypt('p'.codeUnits, 's'.codeUnits, n: n, r: 1, p: 1, dkLen: 32),
          throwsA(equals(const ScryptInvalidN())),
          reason: 'n=$n',
        );
      }
    });

    test('n too large', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: (1 << 20) + 1, r: 1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptNTooLarge())),
      );
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: 1 << 21, r: 1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptNTooLarge())),
      );
    });

    test('n exactly at limit accepted', () {
      // N = 2^20 with r=1, p=1: 128 MiB working set. Slow by design;
      // the generous timeout keeps it in the default suite.
      final result = scrypt('p'.codeUnits, 's'.codeUnits,
          n: 1 << 20, r: 1, p: 1, dkLen: 1);
      expect(result.length, equals(1));
    }, timeout: Timeout(Duration(minutes: 10)));

    test('r zero', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: 0, p: 1, dkLen: 32),
        throwsA(equals(const ScryptInvalidR())),
      );
    });

    test('p zero', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: 1, p: 0, dkLen: 32),
        throwsA(equals(const ScryptInvalidP())),
      );
    });

    test('dk len zero', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: 1, p: 1, dkLen: 0),
        throwsA(equals(const ScryptInvalidKeyLength())),
      );
    });

    test('dk len too large', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: 2, r: 1, p: 1, dkLen: (1 << 20) + 1),
        throwsA(equals(const ScryptKeyLengthTooLarge())),
      );
    });

    test('pr too large', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: 2, r: 1 << 15, p: 1 << 16, dkLen: 32),
        throwsA(equals(const ScryptPRTooLarge())),
      );
    });

    test('b len too large', () {
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: 2, r: 1 << 24, p: 1, dkLen: 32),
        throwsA(equals(const ScryptPRTooLarge())),
      );
    });

    test('pr exactly at limit accepted', () {
      final result =
          scrypt('pw'.codeUnits, 's'.codeUnits, n: 2, r: 1, p: 1, dkLen: 1);
      expect(result.length, equals(1));
    });
  });

  group('lane extras', () {
    test('negative counts rejected', () {
      expect(
        () =>
            scrypt('p'.codeUnits, 's'.codeUnits, n: -2, r: 1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptInvalidN())),
      );
      expect(
        () =>
            scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: -1, p: 1, dkLen: 32),
        throwsA(equals(const ScryptInvalidR())),
      );
      expect(
        () =>
            scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: 1, p: -1, dkLen: 32),
        throwsA(equals(const ScryptInvalidP())),
      );
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits, n: 2, r: 1, p: 1, dkLen: -1),
        throwsA(equals(const ScryptInvalidKeyLength())),
      );
    });

    test('oversized working buffer surfaces pbkdf2 failure', () {
      // bLen = 2^21 passes scrypt's own caps (p*r = 2^14) but exceeds
      // PBKDF2's 2^20 output cap, mapping to ScryptHmacError.
      expect(
        () => scrypt('p'.codeUnits, 's'.codeUnits,
            n: 2, r: 1 << 14, p: 1, dkLen: 32),
        throwsA(equals(const ScryptHmacError())),
      );
    });

    test('exact error messages', () {
      expect(const ScryptInvalidN().toString(),
          equals('scrypt: N must be a power of two >= 2'));
      expect(const ScryptNTooLarge().toString(),
          equals('scrypt: N must not exceed 2^20'));
      expect(const ScryptInvalidR().toString(),
          equals('scrypt: r must be positive'));
      expect(const ScryptInvalidP().toString(),
          equals('scrypt: p must be positive'));
      expect(const ScryptInvalidKeyLength().toString(),
          equals('scrypt: dk_len must be positive'));
      expect(const ScryptKeyLengthTooLarge().toString(),
          equals('scrypt: dk_len must not exceed 2^20'));
      expect(const ScryptPRTooLarge().toString(),
          equals('scrypt: p * r must not exceed 2^30'));
      expect(const ScryptHmacError().toString(),
          equals('scrypt: internal PBKDF2 computation failed'));
    });

    test('error values compare by type', () {
      final errors = [
        const ScryptInvalidN(),
        const ScryptNTooLarge(),
        const ScryptInvalidR(),
        const ScryptInvalidP(),
        const ScryptInvalidKeyLength(),
        const ScryptKeyLengthTooLarge(),
        const ScryptPRTooLarge(),
        const ScryptHmacError(),
      ];
      for (final error in errors) {
        expect(error == error, isTrue);
        expect(error.hashCode, equals(error.hashCode));
      }
      for (var i = 0; i < errors.length; i++) {
        for (var j = 0; j < errors.length; j++) {
          if (i != j) {
            expect(errors[i] == errors[j], isFalse, reason: '$i vs $j');
          }
        }
      }
    });
  });
}
