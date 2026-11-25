/// Unit tests for `coding_adventures_pbkdf2`: all 21 reference
/// tests mirrored case-for-case (RFC 6070 SHA-1, RFC 7914 SHA-256,
/// SHA-512 sanity, hex variants, validation, binding), plus lane
/// extras (negative counts, exact error texts, value equality).
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbkdf2/pbkdf2.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('RFC 6070 sha1', () {
    test('c1', () {
      final dk = pbkdf2HmacSha1('password'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 20);
      expect(dk, equals(_unhex('0c60c80f961f0e71f3a9b524af6012062fe037a6')));
    });

    test('c4096', () {
      final dk = pbkdf2HmacSha1('password'.codeUnits, 'salt'.codeUnits,
          iterations: 4096, keyLength: 20);
      expect(dk, equals(_unhex('4b007901b765489abead49d926f721d065a429c1')));
    });

    test('long password salt', () {
      final dk = pbkdf2HmacSha1('passwordPASSWORDpassword'.codeUnits,
          'saltSALTsaltSALTsaltSALTsaltSALTsalt'.codeUnits,
          iterations: 4096, keyLength: 25);
      expect(dk,
          equals(_unhex('3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038')));
    });

    test('null bytes', () {
      // NUL bytes via explicit byte lists: no escape in the source
      // can silently mis-encode them.
      final password =
          Uint8List.fromList([...'pass'.codeUnits, 0, ...'word'.codeUnits]);
      final salt =
          Uint8List.fromList([...'sa'.codeUnits, 0, ...'lt'.codeUnits]);
      final dk =
          pbkdf2HmacSha1(password, salt, iterations: 4096, keyLength: 16);
      expect(dk, equals(_unhex('56fa6aa75548099dcc37d7f03425e0c3')));
    });
  });

  group('RFC 7914 sha256', () {
    test('c1 64 bytes', () {
      final dk = pbkdf2HmacSha256('passwd'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 64);
      expect(
          dk,
          equals(_unhex(
              '55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783')));
    });

    test('output length', () {
      final dk = pbkdf2HmacSha256('key'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(dk.length, equals(32));
    });

    test('truncation consistency', () {
      final short = pbkdf2HmacSha256('key'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 16);
      final full = pbkdf2HmacSha256('key'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(short, equals(full.sublist(0, 16)));
    });

    test('multi block', () {
      final dk64 = pbkdf2HmacSha256('password'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 64);
      final dk32 = pbkdf2HmacSha256('password'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(dk64.length, equals(64));
      expect(dk64.sublist(0, 32), equals(dk32));
    });
  });

  group('sha512', () {
    test('output length', () {
      final dk = pbkdf2HmacSha512('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 1, keyLength: 64);
      expect(dk.length, equals(64));
    });

    test('truncation', () {
      final short = pbkdf2HmacSha512('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 1, keyLength: 32);
      final full = pbkdf2HmacSha512('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 1, keyLength: 64);
      expect(short, equals(full.sublist(0, 32)));
    });
  });

  group('hex variants', () {
    test('sha1 rfc6070', () {
      expect(
        pbkdf2HmacSha1Hex('password'.codeUnits, 'salt'.codeUnits,
            iterations: 1, keyLength: 20),
        equals('0c60c80f961f0e71f3a9b524af6012062fe037a6'),
      );
    });

    test('sha256 matches bytes', () {
      final dk = pbkdf2HmacSha256('passwd'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      final h = pbkdf2HmacSha256Hex('passwd'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(h, equals(_hex(dk)));
    });

    test('sha512 matches bytes', () {
      final dk = pbkdf2HmacSha512('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 1, keyLength: 64);
      final h = pbkdf2HmacSha512Hex('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 1, keyLength: 64);
      expect(h, equals(_hex(dk)));
    });
  });

  group('validation', () {
    test('empty password returns error', () {
      expect(
        () => pbkdf2HmacSha256([], 'salt'.codeUnits,
            iterations: 1, keyLength: 32),
        throwsA(equals(const Pbkdf2EmptyPassword())),
      );
      expect(
        () =>
            pbkdf2HmacSha1([], 'salt'.codeUnits, iterations: 1, keyLength: 20),
        throwsA(equals(const Pbkdf2EmptyPassword())),
      );
      expect(
        () => pbkdf2HmacSha512([], 'salt'.codeUnits,
            iterations: 1, keyLength: 64),
        throwsA(equals(const Pbkdf2EmptyPassword())),
      );
    });

    test('empty password allowed when flag set', () {
      expect(
          pbkdf2HmacSha256([], 'salt'.codeUnits,
                  iterations: 1, keyLength: 32, allowEmptyPassword: true)
              .length,
          equals(32));
      expect(
          pbkdf2HmacSha1([], 'salt'.codeUnits,
                  iterations: 1, keyLength: 20, allowEmptyPassword: true)
              .length,
          equals(20));
      expect(
          pbkdf2HmacSha512([], 'salt'.codeUnits,
                  iterations: 1, keyLength: 64, allowEmptyPassword: true)
              .length,
          equals(64));
    });

    test('zero iterations returns error', () {
      expect(
        () => pbkdf2HmacSha256('pw'.codeUnits, 'salt'.codeUnits,
            iterations: 0, keyLength: 32),
        throwsA(equals(const Pbkdf2InvalidIterations())),
      );
    });

    test('zero key length returns error', () {
      expect(
        () => pbkdf2HmacSha256('pw'.codeUnits, 'salt'.codeUnits,
            iterations: 1, keyLength: 0),
        throwsA(equals(const Pbkdf2InvalidKeyLength())),
      );
    });

    test('key length over 2^20 rejected', () {
      expect(
        () => pbkdf2HmacSha256('pw'.codeUnits, 'salt'.codeUnits,
            iterations: 1, keyLength: (1 << 20) + 1),
        throwsA(equals(const Pbkdf2KeyLengthTooLarge())),
      );
    });

    test('empty salt is allowed', () {
      final dk = pbkdf2HmacSha256('password'.codeUnits, [],
          iterations: 1, keyLength: 32);
      expect(dk.length, equals(32));
    });
  });

  group('binding', () {
    test('deterministic', () {
      Uint8List run() => pbkdf2HmacSha256('secret'.codeUnits, 'nacl'.codeUnits,
          iterations: 100, keyLength: 32);
      expect(run(), equals(run()));
    });

    test('different salts', () {
      final a = pbkdf2HmacSha256('password'.codeUnits, 'salt1'.codeUnits,
          iterations: 1, keyLength: 32);
      final b = pbkdf2HmacSha256('password'.codeUnits, 'salt2'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(a, isNot(equals(b)));
    });

    test('different passwords', () {
      final a = pbkdf2HmacSha256('password1'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      final b = pbkdf2HmacSha256('password2'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      expect(a, isNot(equals(b)));
    });

    test('different iterations', () {
      final a = pbkdf2HmacSha256('password'.codeUnits, 'salt'.codeUnits,
          iterations: 1, keyLength: 32);
      final b = pbkdf2HmacSha256('password'.codeUnits, 'salt'.codeUnits,
          iterations: 2, keyLength: 32);
      expect(a, isNot(equals(b)));
    });
  });

  group('lane extras', () {
    test('negative counts rejected', () {
      expect(
        () => pbkdf2HmacSha256('pw'.codeUnits, 'salt'.codeUnits,
            iterations: -1, keyLength: 32),
        throwsA(equals(const Pbkdf2InvalidIterations())),
      );
      expect(
        () => pbkdf2HmacSha512('pw'.codeUnits, 'salt'.codeUnits,
            iterations: 1, keyLength: -8),
        throwsA(equals(const Pbkdf2InvalidKeyLength())),
      );
    });

    test('exact error messages', () {
      expect(const Pbkdf2EmptyPassword().toString(),
          equals('pbkdf2: password must not be empty'));
      expect(const Pbkdf2InvalidIterations().toString(),
          equals('pbkdf2: iterations must be positive'));
      expect(const Pbkdf2InvalidKeyLength().toString(),
          equals('pbkdf2: key_length must be positive'));
      expect(const Pbkdf2KeyLengthTooLarge().toString(),
          equals('pbkdf2: key_length must not exceed 2^20 (1 MiB)'));
      expect(const Pbkdf2PrfError().toString(),
          equals('pbkdf2: pseudorandom function returned an error'));
    });

    test('error values compare by type', () {
      final errors = [
        const Pbkdf2EmptyPassword(),
        const Pbkdf2InvalidIterations(),
        const Pbkdf2InvalidKeyLength(),
        const Pbkdf2KeyLengthTooLarge(),
        const Pbkdf2PrfError(),
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
