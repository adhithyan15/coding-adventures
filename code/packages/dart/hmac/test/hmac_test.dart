/// Unit tests for `coding_adventures_hmac`, mirroring the Rust
/// reference suite test-for-test: RFC 4231 (SHA-256/384/512) and RFC
/// 2202 (MD5/SHA-1) vectors, fixed-memory-vs-generic agreement across
/// block boundaries, segmented-message equivalence, error categories,
/// return lengths, and authentication properties (determinism,
/// sensitivity, constant-time verification).
library;

import 'dart:typed_data';

import 'package:coding_adventures_hmac/hmac.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256lane;
import 'package:coding_adventures_sha384/sha384.dart' as sha384lane;
import 'package:test/test.dart';

Uint8List _bytes(List<int> data) => Uint8List.fromList(data);

Uint8List _fill(int length, int byte) =>
    Uint8List.fromList(List<int>.filled(length, byte));

void main() {
  group('RFC 4231 HMAC-SHA256', () {
    test('tc1', () {
      expect(
        hmacSha256Hex(_fill(20, 0x0b), 'Hi There'.codeUnits),
        'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7',
      );
    });

    test('tc2', () {
      expect(
        hmacSha256Hex(
            'Jefe'.codeUnits, 'what do ya want for nothing?'.codeUnits),
        '5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843',
      );
    });

    test('tc3', () {
      expect(
        hmacSha256Hex(_fill(20, 0xaa), _fill(50, 0xdd)),
        '773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe',
      );
    });

    test('tc6 long key', () {
      expect(
        hmacSha256Hex(_fill(131, 0xaa),
            'Test Using Larger Than Block-Size Key - Hash Key First'.codeUnits),
        '60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54',
      );
    });

    test('tc7', () {
      expect(
        hmacSha256Hex(
          _fill(131, 0xaa),
          'This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.'
              .codeUnits,
        ),
        '9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2',
      );
    });
  });

  group('fixed-memory SHA-256', () {
    test('matches generic reference across boundaries', () {
      for (final keyLength in [1, 32, 63, 64, 65, 128, 129]) {
        final key = _fill(keyLength, 0xa5);
        for (final messageLength in [0, 1, 55, 56, 63, 64, 65, 127, 128, 129]) {
          final message = _fill(messageLength, 0x5a);
          expect(
            hmacSha256(key, message),
            equals(hmac(sha256lane.sha256, 64, key, message)),
            reason: 'key $keyLength, message $messageLength',
          );
        }
      }
    });

    test('segmented matches one-shot across boundaries', () {
      for (final messageLength in [0, 1, 55, 56, 63, 64, 65, 127, 128, 129]) {
        final message = Uint8List.fromList(
          List<int>.generate(messageLength, (index) => index % 251),
        );
        final expected = hmacSha256('segmented-key'.codeUnits, message);
        for (var firstSplit = 0; firstSplit <= messageLength; firstSplit++) {
          final secondSplit = firstSplit + (messageLength - firstSplit) ~/ 2;
          final segments = [
            message.sublist(0, firstSplit),
            <int>[],
            message.sublist(firstSplit, secondSplit),
            message.sublist(secondSplit),
          ];
          expect(
            hmacSha256Segments('segmented-key'.codeUnits, segments),
            equals(expected),
            reason: 'message $messageLength, first split $firstSplit',
          );
        }
      }
    });

    test('segmented empty list is empty message', () {
      expect(
        hmacSha256Segments('key'.codeUnits, []),
        equals(hmacSha256('key'.codeUnits, <int>[])),
      );
    });
  });

  group('errors', () {
    test('categories are payload free', () {
      expect(const HmacEmptyKey().toString(), 'HMAC key must not be empty');
      expect(const HmacInputTooLong().toString(), 'HMAC input is too long');
    });

    test('equality distinguishes categories', () {
      expect(const HmacEmptyKey(), equals(const HmacEmptyKey()));
      expect(const HmacInputTooLong(), equals(const HmacInputTooLong()));
      expect(const HmacEmptyKey(), isNot(equals(const HmacInputTooLong())));
      expect(const HmacEmptyKey().hashCode,
          isNot(equals(const HmacInputTooLong().hashCode)));
    });

    test('empty key throws on every variant', () {
      expect(() => hmacSha256([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha384([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha512([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacMd5([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha1([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(
        () => hmacSha256Segments([], [
          [1]
        ]),
        throwsA(isA<HmacEmptyKey>()),
      );
      expect(
        () => hmacSha384Segments([], [
          [1]
        ]),
        throwsA(isA<HmacEmptyKey>()),
      );
    });

    test('empty key throws on every hex variant', () {
      expect(() => hmacMd5Hex([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha1Hex([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha256Hex([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha384Hex([], [1]), throwsA(isA<HmacEmptyKey>()));
      expect(() => hmacSha512Hex([], [1]), throwsA(isA<HmacEmptyKey>()));
    });
  });

  group('RFC 4231 HMAC-SHA384', () {
    test('tc1', () {
      expect(
        hmacSha384Hex(_fill(20, 0x0b), 'Hi There'.codeUnits),
        'afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59cfaea9ea9076ede7f4af152e8b2fa9cb6',
      );
    });

    test('tc2', () {
      expect(
        hmacSha384Hex(
            'Jefe'.codeUnits, 'what do ya want for nothing?'.codeUnits),
        'af45d2e376484031617f78d2b58a6b1b9c7ef464f5a01b47e42ec3736322445e8e2240ca5e69e2c78b3239ecfab21649',
      );
    });

    test('tc3', () {
      expect(
        hmacSha384Hex(_fill(20, 0xaa), _fill(50, 0xdd)),
        '88062608d3e6ad8a0aa2ace014c8a86f0aa635d947ac9febe83ef4e55966144b2a5ab39dc13814b94e3ab6e101a34f27',
      );
    });

    test('tc6 long key', () {
      expect(
        hmacSha384Hex(_fill(131, 0xaa),
            'Test Using Larger Than Block-Size Key - Hash Key First'.codeUnits),
        '4ece084485813e9088d2c63a041bc5b44f9ef1012a2b588f3cd11f05033ac4c60c2ef6ab4030fe8296248df163f44952',
      );
    });

    test('tc7', () {
      expect(
        hmacSha384Hex(
          _fill(131, 0xaa),
          'This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.'
              .codeUnits,
        ),
        '6617178e941f020d351e2f254e8fd32c602420feb0b8fb9adccebb82461e99c5a678cc31e799176d3860e6110c46523e',
      );
    });
  });

  group('fixed-memory SHA-384', () {
    test('matches generic reference across boundaries', () {
      for (final keyLength in [1, 48, 127, 128, 129, 256, 257]) {
        final key = _fill(keyLength, 0xa5);
        for (final messageLength in [
          0,
          1,
          111,
          112,
          127,
          128,
          129,
          255,
          256,
          257
        ]) {
          final message = _fill(messageLength, 0x5a);
          expect(
            hmacSha384(key, message),
            equals(hmac(sha384lane.sha384, 128, key, message)),
            reason: 'key $keyLength, message $messageLength',
          );
        }
      }
    });

    test('segmented matches one-shot across boundaries', () {
      for (final messageLength in [
        0,
        1,
        111,
        112,
        127,
        128,
        129,
        255,
        256,
        257
      ]) {
        final message = Uint8List.fromList(
          List<int>.generate(messageLength, (index) => index % 251),
        );
        final expected = hmacSha384('segmented-key'.codeUnits, message);
        for (var firstSplit = 0; firstSplit <= messageLength; firstSplit++) {
          final secondSplit = firstSplit + (messageLength - firstSplit) ~/ 2;
          final segments = [
            message.sublist(0, firstSplit),
            <int>[],
            message.sublist(firstSplit, secondSplit),
            message.sublist(secondSplit),
          ];
          expect(
            hmacSha384Segments('segmented-key'.codeUnits, segments),
            equals(expected),
            reason: 'message $messageLength, first split $firstSplit',
          );
        }
      }
    });

    test('segmented empty list is empty message', () {
      expect(
        hmacSha384Segments('key'.codeUnits, []),
        equals(hmacSha384('key'.codeUnits, <int>[])),
      );
    });
  });

  group('RFC 4231 HMAC-SHA512', () {
    test('tc1', () {
      expect(
        hmacSha512Hex(_fill(20, 0x0b), 'Hi There'.codeUnits),
        '87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854',
      );
    });

    test('tc2', () {
      expect(
        hmacSha512Hex(
            'Jefe'.codeUnits, 'what do ya want for nothing?'.codeUnits),
        '164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737',
      );
    });

    test('tc6 long key', () {
      expect(
        hmacSha512Hex(_fill(131, 0xaa),
            'Test Using Larger Than Block-Size Key - Hash Key First'.codeUnits),
        '80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598',
      );
    });
  });

  group('RFC 2202 HMAC-MD5', () {
    test('tc1', () {
      expect(
        hmacMd5Hex(_fill(16, 0x0b), 'Hi There'.codeUnits),
        '9294727a3638bb1c13f48ef8158bfc9d',
      );
    });

    test('tc2', () {
      expect(
        hmacMd5Hex('Jefe'.codeUnits, 'what do ya want for nothing?'.codeUnits),
        '750c783e6ab0b503eaa86e310a5db738',
      );
    });

    test('tc6 long key', () {
      expect(
        hmacMd5Hex(_fill(80, 0xaa),
            'Test Using Larger Than Block-Size Key - Hash Key First'.codeUnits),
        '6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd',
      );
    });
  });

  group('RFC 2202 HMAC-SHA1', () {
    test('tc1', () {
      expect(
        hmacSha1Hex(_fill(20, 0x0b), 'Hi There'.codeUnits),
        'b617318655057264e28bc0b6fb378c8ef146be00',
      );
    });

    test('tc2', () {
      expect(
        hmacSha1Hex('Jefe'.codeUnits, 'what do ya want for nothing?'.codeUnits),
        'effcdf6ae5eb2fa2d27416d5f184df9c259a7c79',
      );
    });

    test('tc6 long key', () {
      expect(
        hmacSha1Hex(_fill(80, 0xaa),
            'Test Using Larger Than Block-Size Key - Hash Key First'.codeUnits),
        'aa4ae5e15272d00e95705637ce8a3b55ed402112',
      );
    });
  });

  group('generic hmac key shapes', () {
    test('plain-list keys match typed keys, short and long', () {
      final shortPlain = [11, 22, 33];
      final shortTyped = Uint8List.fromList(shortPlain);
      final longPlain = List<int>.filled(100, 0x7e);
      final longTyped = Uint8List.fromList(longPlain);
      final msg = [1, 2, 3];
      expect(
        hmac(sha256lane.sha256, 64, shortPlain, msg),
        equals(hmac(sha256lane.sha256, 64, shortTyped, msg)),
      );
      expect(
        hmac(sha256lane.sha256, 64, longPlain, msg),
        equals(hmac(sha256lane.sha256, 64, longTyped, msg)),
      );
    });
  });

  group('lengths and keys', () {
    test('return lengths', () {
      expect(hmacMd5([107], [109]).length, 16);
      expect(hmacSha1([107], [109]).length, 20);
      expect(hmacSha256([107], [109]).length, 32);
      expect(hmacSha384([107], [109]).length, 48);
      expect(hmacSha512([107], [109]).length, 64);
    });

    test('empty message is allowed', () {
      expect(hmacSha256([107], <int>[]).length, 32);
      expect(hmacSha384([107], <int>[]).length, 48);
      expect(hmacSha512([107], <int>[]).length, 64);
      expect(hmacMd5([107], <int>[]).length, 16);
      expect(hmacSha1([107], <int>[]).length, 20);
    });

    test('key longer than block hashed', () {
      final k65 = _fill(65, 0x01);
      final k66 = _fill(66, 0x01);
      expect(
        hmacSha256(k65, [109]),
        isNot(equals(hmacSha256(k66, [109]))),
      );
    });
  });

  group('authentication properties', () {
    test('deterministic', () {
      expect(
        hmacSha256([107], [109]),
        equals(hmacSha256([107], [109])),
      );
    });

    test('verify accepts identical tags', () {
      final tag = hmacSha256('key'.codeUnits, 'message'.codeUnits);
      expect(verifyHmac(tag, tag), isTrue);
    });

    test('verify rejects a difference at every position', () {
      final expected = hmacSha256('key'.codeUnits, 'message'.codeUnits);
      for (var index = 0; index < expected.length; index++) {
        final actual = _bytes(expected)..[index] ^= 1;
        expect(verifyHmac(expected, actual), isFalse,
            reason: 'difference at index $index');
      }
    });

    test('verify rejects public length mismatches', () {
      expect(
        verifyHmac('fixed-length-tag'.codeUnits, 'fixed-length-ta'.codeUnits),
        isFalse,
      );
      expect(
        verifyHmac('fixed-length-tag'.codeUnits, 'fixed-length-tag!'.codeUnits),
        isFalse,
      );
    });

    test('key sensitivity', () {
      expect(
        hmacSha256([107, 49], [109]),
        isNot(equals(hmacSha256([107, 50], [109]))),
      );
    });

    test('message sensitivity', () {
      expect(
        hmacSha256([107], [109, 49]),
        isNot(equals(hmacSha256([107], [109, 50]))),
      );
    });

    test('hex matches bytes on every variant', () {
      final key = [107];
      final message = [109];
      expect(hmacMd5Hex(key, message), _hexOf(hmacMd5(key, message)));
      expect(hmacSha1Hex(key, message), _hexOf(hmacSha1(key, message)));
      expect(hmacSha256Hex(key, message), _hexOf(hmacSha256(key, message)));
      expect(hmacSha384Hex(key, message), _hexOf(hmacSha384(key, message)));
      expect(hmacSha512Hex(key, message), _hexOf(hmacSha512(key, message)));
    });
  });
}

String _hexOf(List<int> bytes) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in bytes) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}
