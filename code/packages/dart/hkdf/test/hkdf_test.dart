/// Unit tests for `coding_adventures_hkdf`: all 45 reference tests
/// mirrored case-for-case (fixed extract/expand incl. import and
/// redaction, RFC 5869 vectors, edge cases, BLAKE2b structural
/// tests), plus lane extras (negative lengths, dispose semantics,
/// exact error texts, value equality).
///
/// One reference assertion has no Dart equivalent: the fixed-import
/// test reads the private 48-byte owner's SHA-256 tail, which is
/// unobservable through the public API (Dart privacy is
/// library-scoped). The zero-initialized owner plus prefix-only
/// writes make it structural.
library;

import 'dart:typed_data';

import 'package:coding_adventures_hkdf/hkdf.dart';
import 'package:coding_adventures_hmac/hmac.dart' as hmaclane;
import 'package:test/test.dart';

Uint8List _h(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Uint8List _referenceExpandSha384(List<int> prk, List<int> info, int length) {
  final output = <int>[];
  var previous = <int>[];
  final blocks = (length + 47) ~/ 48;
  for (var counter = 1; counter <= blocks; counter++) {
    final message = [...previous, ...info, counter];
    previous = hmaclane.hmacSha384(prk, message);
    output.addAll(previous);
  }
  return Uint8List.fromList(output.sublist(0, length));
}

void main() {
  group('fixed extract', () {
    test('sha256 matches RFC 5869 TC1', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      final prk = hkdfExtractFixed(salt, ikm, FixedHkdfHash.sha256);
      try {
        expect(prk.algorithm, equals(FixedHkdfHash.sha256));
        expect(
          _hex(prk.asBytes()),
          equals(
              '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5'),
        );
      } finally {
        prk.dispose();
      }
    });

    test('sha384 matches RFC 4231 TC2 PRF', () {
      final prk = hkdfExtractFixed(
        'Jefe'.codeUnits,
        'what do ya want for nothing?'.codeUnits,
        FixedHkdfHash.sha384,
      );
      try {
        expect(prk.algorithm, equals(FixedHkdfHash.sha384));
        expect(
          _hex(prk.asBytes()),
          equals(
              'af45d2e376484031617f78d2b58a6b1b9c7ef464f5a01b47e42ec3736322445e8e2240ca5e69e2c78b3239ecfab21649'),
        );
      } finally {
        prk.dispose();
      }
    });

    test('uses hash-len zero default salt', () {
      final sha256 =
          hkdfExtractFixed([], 'ikm'.codeUnits, FixedHkdfHash.sha256);
      try {
        final legacy = hkdfExtract([], 'ikm'.codeUnits, HashAlgorithm.sha256);
        expect(sha256.asBytes(), equals(legacy));
      } finally {
        sha256.dispose();
      }

      final sha384 =
          hkdfExtractFixed([], 'ikm'.codeUnits, FixedHkdfHash.sha384);
      try {
        final expected = hmaclane.hmacSha384(Uint8List(48), 'ikm'.codeUnits);
        expect(sha384.asBytes(), equals(expected));
      } finally {
        sha384.dispose();
      }
    });
  });

  group('fixed PRK import', () {
    test('accepts exact sha256 and sha384 lengths', () {
      for (final algorithm in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final bytes = Uint8List.fromList(List<int>.generate(
            algorithm.hashLen, (index) => (index * 17 + 3) & 0xff));
        final imported = HkdfPrk.importExact(algorithm, bytes);
        try {
          expect(imported.algorithm, equals(algorithm));
          expect(imported.asBytes(), equals(bytes));
        } finally {
          imported.dispose();
        }
      }
    });

    test('rejects every non-exact boundary without secret data', () {
      for (final algorithm in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final expected = algorithm.hashLen;
        for (final actual in [0, 1, expected - 1, expected + 1, 49]) {
          final bytes = Uint8List.fromList(List<int>.filled(actual, 0xa5));
          try {
            HkdfPrk.importExact(algorithm, bytes);
            fail('length $actual accepted for $algorithm');
          } on HkdfPrkImportError catch (e) {
            expect(e, equals(const HkdfPrkImportError()));
            expect(
              e.toString(),
              equals(
                  'HKDF pseudorandom key must exactly match the selected hash length'),
            );
            expect(e.toString().contains(actual.toString()), isFalse);
            expect(e.toString().contains('a5'), isFalse);
          }
        }
      }
    });

    test('import expands identically to the original owner', () {
      for (final algorithm in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final extracted =
            hkdfExtractFixed('salt'.codeUnits, 'ikm'.codeUnits, algorithm);
        try {
          final imported = HkdfPrk.importExact(algorithm, extracted.asBytes());
          try {
            final extractedOutput = Uint8List(80);
            final importedOutput = Uint8List(80);
            hkdfExpandFixed(
                extracted, 'import equivalence'.codeUnits, extractedOutput);
            hkdfExpandFixed(
                imported, 'import equivalence'.codeUnits, importedOutput);
            expect(importedOutput, equals(extractedOutput));
          } finally {
            imported.dispose();
          }
        } finally {
          extracted.dispose();
        }
      }
    });

    test('debug redacts secret bytes', () {
      final prk = hkdfExtractFixed(
          'salt'.codeUnits, 'secret-ikm'.codeUnits, FixedHkdfHash.sha256);
      try {
        final rendered = prk.toString();
        expect(rendered.contains('sha256'), isTrue);
        expect(rendered.contains('32'), isTrue);
        expect(rendered.contains(_hex(prk.asBytes())), isFalse);
      } finally {
        prk.dispose();
      }
    });
  });

  group('fixed expand', () {
    test('sha256 matches RFC 5869 TC1', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      final info = _h('f0f1f2f3f4f5f6f7f8f9');
      final prk = hkdfExtractFixed(salt, ikm, FixedHkdfHash.sha256);
      try {
        final output = Uint8List(42);
        hkdfExpandFixed(prk, info, output);
        expect(
          _hex(output),
          equals(
              '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'),
        );
      } finally {
        prk.dispose();
      }
    });

    test('sha384 matches independent vector', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      final info = _h('f0f1f2f3f4f5f6f7f8f9');
      final prk = hkdfExtractFixed(salt, ikm, FixedHkdfHash.sha384);
      try {
        final output = Uint8List(82);
        hkdfExpandFixed(prk, info, output);
        expect(
          _hex(output),
          equals(
              '9b5097a86038b805309076a44b3a9f38063e25b516dcbf369f394cfab43685f748b6457763e4f0204fc5d95d1da3e62587b22eb8943d0fab6bb631a2fe9df1a68c6ce5d56116a52005b3f122b88b39b7251f'),
        );
      } finally {
        prk.dispose();
      }
    });

    test('sha256 empty info matches RFC 5869 TC3', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final prk = hkdfExtractFixed([], ikm, FixedHkdfHash.sha256);
      try {
        final output = Uint8List(42);
        hkdfExpandFixed(prk, [], output);
        expect(
          _hex(output),
          equals(
              '8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8'),
        );
      } finally {
        prk.dispose();
      }
    });

    test('matches compatibility references across output boundaries', () {
      final info = 'fixed expand boundary context'.codeUnits;
      for (final algorithm in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final prk =
            hkdfExtractFixed('salt'.codeUnits, 'ikm'.codeUnits, algorithm);
        try {
          final hashLen = algorithm.hashLen;
          for (final length in [
            1,
            hashLen - 1,
            hashLen,
            hashLen + 1,
            2 * hashLen,
            2 * hashLen + 1,
            255 * hashLen,
          ]) {
            final actual = Uint8List.fromList(List<int>.filled(length, 0xa5));
            hkdfExpandFixed(prk, info, actual);
            final Uint8List expected;
            if (algorithm == FixedHkdfHash.sha256) {
              expected =
                  hkdfExpand(prk.asBytes(), info, length, HashAlgorithm.sha256);
            } else {
              expected = _referenceExpandSha384(prk.asBytes(), info, length);
            }
            expect(actual, equals(expected),
                reason: '$algorithm output length $length');
          }
        } finally {
          prk.dispose();
        }
      }
    });

    test('length errors do not modify caller output', () {
      for (final algorithm in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final prk =
            hkdfExtractFixed('salt'.codeUnits, 'ikm'.codeUnits, algorithm);
        try {
          expect(
            () => hkdfExpandFixed(prk, 'info'.codeUnits, Uint8List(0)),
            throwsA(equals(const HkdfOutputTooShort())),
          );
          final maximum = 255 * algorithm.hashLen;
          final oversized =
              Uint8List.fromList(List<int>.filled(maximum + 1, 0xa5));
          expect(
            () => hkdfExpandFixed(prk, 'info'.codeUnits, oversized),
            throwsA(equals(
                HkdfOutputTooLong(requested: maximum + 1, maximum: maximum))),
          );
          expect(oversized.every((byte) => byte == 0xa5), isTrue);
        } finally {
          prk.dispose();
        }
      }
    });
  });

  group('RFC 5869 TC1', () {
    test('extract', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      expect(
        _hex(hkdfExtract(salt, ikm, HashAlgorithm.sha256)),
        equals(
            '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5'),
      );
    });

    test('expand', () {
      final prk = _h(
          '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5');
      final info = _h('f0f1f2f3f4f5f6f7f8f9');
      expect(
        _hex(hkdfExpand(prk, info, 42, HashAlgorithm.sha256)),
        equals(
            '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'),
      );
    });

    test('combined', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      final info = _h('f0f1f2f3f4f5f6f7f8f9');
      expect(
        _hex(hkdf(salt, ikm, info, 42, HashAlgorithm.sha256)),
        equals(
            '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'),
      );
    });
  });

  group('RFC 5869 TC2', () {
    test('extract', () {
      final ikm = Uint8List.fromList(List<int>.generate(80, (i) => i));
      final salt = Uint8List.fromList(List<int>.generate(80, (i) => 0x60 + i));
      expect(
        _hex(hkdfExtract(salt, ikm, HashAlgorithm.sha256)),
        equals(
            '06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244'),
      );
    });

    test('expand', () {
      final prk = _h(
          '06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244');
      final info = Uint8List.fromList(List<int>.generate(80, (i) => 0xb0 + i));
      expect(
        _hex(hkdfExpand(prk, info, 82, HashAlgorithm.sha256)),
        equals(
            'b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87'),
      );
    });

    test('combined', () {
      final ikm = Uint8List.fromList(List<int>.generate(80, (i) => i));
      final salt = Uint8List.fromList(List<int>.generate(80, (i) => 0x60 + i));
      final info = Uint8List.fromList(List<int>.generate(80, (i) => 0xb0 + i));
      expect(
        _hex(hkdf(salt, ikm, info, 82, HashAlgorithm.sha256)),
        equals(
            'b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87'),
      );
    });
  });

  group('RFC 5869 TC3', () {
    test('extract', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      expect(
        _hex(hkdfExtract([], ikm, HashAlgorithm.sha256)),
        equals(
            '19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04'),
      );
    });

    test('expand', () {
      final prk = _h(
          '19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04');
      expect(
        _hex(hkdfExpand(prk, [], 42, HashAlgorithm.sha256)),
        equals(
            '8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8'),
      );
    });

    test('combined', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      expect(
        _hex(hkdf([], ikm, [], 42, HashAlgorithm.sha256)),
        equals(
            '8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8'),
      );
    });
  });

  group('edge cases', () {
    test('expand exactly hash len', () {
      final prk = _h(
          '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5');
      final okm = hkdfExpand(prk, 'test'.codeUnits, 32, HashAlgorithm.sha256);
      expect(okm.length, equals(32));
    });

    test('expand one byte', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      final okm = hkdfExpand(prk, [], 1, HashAlgorithm.sha256);
      expect(okm.length, equals(1));
    });

    test('expand max length sha256', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      final okm = hkdfExpand(prk, [], 255 * 32, HashAlgorithm.sha256);
      expect(okm.length, equals(8160));
    });

    test('expand exceeds max length', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      expect(
        () => hkdfExpand(prk, [], 255 * 32 + 1, HashAlgorithm.sha256),
        throwsA(isA<HkdfOutputTooLong>()),
      );
    });

    test('expand zero length', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      expect(
        () => hkdfExpand(prk, [], 0, HashAlgorithm.sha256),
        throwsA(isA<HkdfOutputTooShort>()),
      );
    });

    test('sha512 basic', () {
      final ikm = _h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
      final salt = _h('000102030405060708090a0b0c');
      final prk = hkdfExtract(salt, ikm, HashAlgorithm.sha512);
      expect(prk.length, equals(64));
      final okm = hkdfExpand(prk, 'info'.codeUnits, 64, HashAlgorithm.sha512);
      expect(okm.length, equals(64));
    });

    test('sha512 empty salt', () {
      final ikm = Uint8List.fromList(List<int>.filled(32, 0xab));
      final prk = hkdfExtract([], ikm, HashAlgorithm.sha512);
      expect(prk.length, equals(64));
    });

    test('sha512 max length', () {
      final prk = Uint8List.fromList(List<int>.filled(64, 0x01));
      final okm = hkdfExpand(prk, [], 255 * 64, HashAlgorithm.sha512);
      expect(okm.length, equals(16320));
    });

    test('sha512 exceeds max', () {
      final prk = Uint8List.fromList(List<int>.filled(64, 0x01));
      expect(
        () => hkdfExpand(prk, [], 255 * 64 + 1, HashAlgorithm.sha512),
        throwsA(isA<HkdfOutputTooLong>()),
      );
    });

    test('different info different okm', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      final okm1 =
          hkdfExpand(prk, 'purpose-a'.codeUnits, 32, HashAlgorithm.sha256);
      final okm2 =
          hkdfExpand(prk, 'purpose-b'.codeUnits, 32, HashAlgorithm.sha256);
      expect(okm1, isNot(equals(okm2)));
    });

    test('different salt different prk', () {
      final ikm = Uint8List.fromList(List<int>.filled(32, 0x01));
      final prk1 = hkdfExtract('salt-1'.codeUnits, ikm, HashAlgorithm.sha256);
      final prk2 = hkdfExtract('salt-2'.codeUnits, ikm, HashAlgorithm.sha256);
      expect(prk1, isNot(equals(prk2)));
    });

    test('deterministic', () {
      Uint8List run() => hkdf('salt'.codeUnits, 'ikm'.codeUnits,
          'info'.codeUnits, 42, HashAlgorithm.sha256);
      expect(run(), equals(run()));
    });

    test('round trip extract expand', () {
      final salt = 'my-salt'.codeUnits;
      final ikm = 'my-input-keying-material'.codeUnits;
      final info = 'my-context'.codeUnits;
      const length = 48;
      final combined = hkdf(salt, ikm, info, length, HashAlgorithm.sha256);
      final prk = hkdfExtract(salt, ikm, HashAlgorithm.sha256);
      final manual = hkdfExpand(prk, info, length, HashAlgorithm.sha256);
      expect(combined, equals(manual));
    });

    test('error display', () {
      final tooLong = HkdfOutputTooLong(requested: 9000, maximum: 8160);
      expect(tooLong.toString().contains('9000'), isTrue);
      expect(tooLong.toString().contains('8160'), isTrue);
      expect(
        tooLong.toString(),
        equals('HKDF output length 9000 exceeds maximum 8160 (255 * HashLen)'),
      );
      expect(
        const HkdfOutputTooShort().toString(),
        equals('HKDF output length must be positive'),
      );
      expect(
        const HkdfInputTooLong().toString(),
        equals('HKDF input exceeds the selected hash length bound'),
      );
      expect(
        const HkdfPrfRejectedKey().toString(),
        equals('HKDF pseudorandom function rejected its key'),
      );
    });
  });

  group('blake2b', () {
    test('extract returns 32 bytes', () {
      final prk = hkdfExtract('signal-salt'.codeUnits,
          'shared-secret-from-ecdh'.codeUnits, HashAlgorithm.blake2b);
      expect(prk.length, equals(32));
    });

    test('expand correct length', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x42));
      for (final len in [1, 16, 32, 64, 128]) {
        final okm =
            hkdfExpand(prk, 'info'.codeUnits, len, HashAlgorithm.blake2b);
        expect(okm.length, equals(len), reason: 'expected $len bytes');
      }
    });

    test('deterministic', () {
      Uint8List run() => hkdf('salt'.codeUnits, 'ikm'.codeUnits,
          'info'.codeUnits, 32, HashAlgorithm.blake2b);
      expect(run(), equals(run()));
    });

    test('info provides domain separation', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      final okm1 =
          hkdfExpand(prk, 'x3dh-root-key'.codeUnits, 32, HashAlgorithm.blake2b);
      final okm2 = hkdfExpand(
          prk, 'double-ratchet-ck'.codeUnits, 32, HashAlgorithm.blake2b);
      expect(okm1, isNot(equals(okm2)));
    });

    test('different salts different prk', () {
      final ikm = 'same-ikm'.codeUnits;
      final prk1 =
          hkdfExtract('salt-alice'.codeUnits, ikm, HashAlgorithm.blake2b);
      final prk2 =
          hkdfExtract('salt-bob'.codeUnits, ikm, HashAlgorithm.blake2b);
      expect(prk1, isNot(equals(prk2)));
    });

    test('empty salt uses zero key', () {
      final prk = hkdfExtract([], 'ikm'.codeUnits, HashAlgorithm.blake2b);
      expect(prk.length, equals(32));
    });

    test('empty prk expand takes the unkeyed fallback', () {
      // Extract normalizes empty salts, so the PRF's empty-key arm is
      // reachable only through expand with an explicitly empty PRK
      // (the reference has no test for this path either).
      Uint8List run() =>
          hkdfExpand([], 'info'.codeUnits, 48, HashAlgorithm.blake2b);
      final a = run();
      expect(a.length, equals(48));
      expect(run(), equals(a));
      final keyed = hkdfExpand(Uint8List.fromList(List<int>.filled(32, 0x01)),
          'info'.codeUnits, 48, HashAlgorithm.blake2b);
      expect(a, isNot(equals(keyed)));
    });

    test('combined equals manual two step', () {
      final salt = 'session-salt'.codeUnits;
      final ikm = 'dh-output-bytes'.codeUnits;
      final info = 'coding_adventures_x3dh_v1'.codeUnits;
      const length = 48;
      final combined = hkdf(salt, ikm, info, length, HashAlgorithm.blake2b);
      final prk = hkdfExtract(salt, ikm, HashAlgorithm.blake2b);
      final manual = hkdfExpand(prk, info, length, HashAlgorithm.blake2b);
      expect(combined, equals(manual));
    });

    test('max output', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      final okm = hkdfExpand(prk, [], 255 * 32, HashAlgorithm.blake2b);
      expect(okm.length, equals(255 * 32));
    });

    test('exceeds max output', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      expect(
        () => hkdfExpand(prk, [], 255 * 32 + 1, HashAlgorithm.blake2b),
        throwsA(isA<HkdfOutputTooLong>()),
      );
    });

    test('output differs from sha256 for same inputs', () {
      final salt = 'salt'.codeUnits;
      final ikm = 'ikm'.codeUnits;
      final info = 'info'.codeUnits;
      final sha = hkdf(salt, ikm, info, 32, HashAlgorithm.sha256);
      final b2 = hkdf(salt, ikm, info, 32, HashAlgorithm.blake2b);
      expect(sha, isNot(equals(b2)));
    });
  });

  group('lane extras', () {
    test('negative expand length rejected', () {
      final prk = Uint8List.fromList(List<int>.filled(32, 0x01));
      expect(
        () => hkdfExpand(prk, [], -1, HashAlgorithm.sha256),
        throwsA(equals(const HkdfOutputTooShort())),
      );
      expect(
        () => hkdf([], 'ikm'.codeUnits, [], -5, HashAlgorithm.blake2b),
        throwsA(equals(const HkdfOutputTooShort())),
      );
    });

    test('prk dispose locks the owner', () {
      final prk = hkdfExtractFixed(
          'salt'.codeUnits, 'ikm'.codeUnits, FixedHkdfHash.sha256);
      expect(prk.isDisposed, isFalse);
      expect(prk.asBytes().length, equals(32));
      prk.dispose();
      expect(prk.isDisposed, isTrue);
      expect(() => prk.asBytes(), throwsStateError);
      prk.dispose();
      expect(prk.isDisposed, isTrue);
    });

    test('error values compare by type and payload', () {
      expect(
        HkdfOutputTooLong(requested: 9000, maximum: 8160),
        equals(HkdfOutputTooLong(requested: 9000, maximum: 8160)),
      );
      expect(
        HkdfOutputTooLong(requested: 9000, maximum: 8160),
        isNot(equals(HkdfOutputTooLong(requested: 9001, maximum: 8160))),
      );
      expect(
        HkdfOutputTooLong(requested: 9000, maximum: 8160).hashCode,
        equals(HkdfOutputTooLong(requested: 9000, maximum: 8160).hashCode),
      );
      expect(
        const HkdfOutputTooShort(),
        equals(const HkdfOutputTooShort()),
      );
      expect(
        const HkdfOutputTooShort().hashCode,
        equals(const HkdfOutputTooShort().hashCode),
      );
      expect(
        const HkdfInputTooLong(),
        equals(const HkdfInputTooLong()),
      );
      expect(
        const HkdfInputTooLong().hashCode,
        equals(const HkdfInputTooLong().hashCode),
      );
      expect(
        const HkdfPrfRejectedKey(),
        equals(const HkdfPrfRejectedKey()),
      );
      expect(
        const HkdfPrfRejectedKey().hashCode,
        equals(const HkdfPrfRejectedKey().hashCode),
      );
      expect(
        const HkdfPrkImportError(),
        equals(const HkdfPrkImportError()),
      );
      expect(
        const HkdfPrkImportError().hashCode,
        equals(const HkdfPrkImportError().hashCode),
      );
      expect(
        const HkdfOutputTooShort(),
        isNot(equals(const HkdfInputTooLong())),
      );
    });

    test('hash lengths are specified', () {
      expect(HashAlgorithm.sha256.hashLen, equals(32));
      expect(HashAlgorithm.sha512.hashLen, equals(64));
      expect(HashAlgorithm.blake2b.hashLen, equals(32));
      expect(FixedHkdfHash.sha256.hashLen, equals(32));
      expect(FixedHkdfHash.sha384.hashLen, equals(48));
    });
  });
}
