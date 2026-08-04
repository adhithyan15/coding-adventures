import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_md5/coding_adventures_md5.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

void main() {
  // ==========================================================================
  // RFC 1321 Appendix A.5 known-answer vectors
  // ==========================================================================
  group('RFC 1321 vectors', () {
    const cases = {
      '': 'd41d8cd98f00b204e9800998ecf8427e',
      'a': '0cc175b9c0f1b6a831c399e269772661',
      'abc': '900150983cd24fb0d6963f7d28e17f72',
      'message digest': 'f96b697d7cb7938d525a2f31aaf161d0',
      'abcdefghijklmnopqrstuvwxyz': 'c3fcd3d76192e4007dfb496cca67e13b',
      'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789':
          'd174ab98d277d9f5a5611c2c9f419d9f',
      '12345678901234567890123456789012345678901234567890123456789012345678901234567890':
          '57edf4a22be3c955ac49da2e2107b67a',
    };
    cases.forEach((input, expected) {
      test('"${input.length > 20 ? '${input.substring(0, 17)}...' : input}"', () {
        expect(hexString(a(input)), equals(expected));
      });
    });
  });

  // ==========================================================================
  // Little-endian byte order (MD5's defining quirk)
  // ==========================================================================
  group('little-endian', () {
    test('digest of "a" has the expected LE byte order', () {
      final d = sumMd5(a('a'));
      expect(d[0], equals(0x0c));
      expect(d[1], equals(0xc1));
      expect(d[2], equals(0x75));
      expect(d[3], equals(0xb9));
    });

    test('bytes 0x00..0xFF hash to the known digest', () {
      final data = List<int>.generate(256, (i) => i);
      expect(hexString(data), equals('e2c865db4162bed963bfaa9ef6ac18f0'));
    });
  });

  // ==========================================================================
  // Output format
  // ==========================================================================
  group('output format', () {
    test('digest is always 16 bytes', () {
      expect(sumMd5(a('')).length, equals(16));
      expect(sumMd5(a('hello world')).length, equals(16));
      expect(sumMd5(List<int>.filled(1000, 0)).length, equals(16));
    });

    test('digest is a Uint8List', () {
      expect(sumMd5(a('abc')), isA<Uint8List>());
    });

    test('hex string is 32 lowercase hex chars', () {
      final h = hexString(a('abc'));
      expect(h.length, equals(32));
      expect(RegExp(r'^[0-9a-f]{32}$').hasMatch(h), isTrue);
    });
  });

  // ==========================================================================
  // Core properties
  // ==========================================================================
  group('properties', () {
    test('deterministic', () {
      expect(sumMd5(a('hello')), equals(sumMd5(a('hello'))));
    });

    test('avalanche: one-char change flips many bits', () {
      final h1 = sumMd5(a('hello'));
      final h2 = sumMd5(a('helo'));
      expect(h1, isNot(equals(h2)));
      var bits = 0;
      for (var i = 0; i < 16; i++) {
        var x = h1[i] ^ h2[i];
        while (x != 0) {
          bits += x & 1;
          x >>= 1;
        }
      }
      expect(bits, greaterThan(20), reason: 'only $bits bits differed');
    });

    test('a null byte differs from the empty message', () {
      expect(sumMd5([0x00]), isNot(equals(sumMd5(a('')))));
    });

    test('every single byte value hashes distinctly', () {
      final seen = <String>{};
      for (var i = 0; i <= 255; i++) {
        seen.add(hexString([i]));
      }
      expect(seen.length, equals(256));
    });
  });

  // ==========================================================================
  // Padding / block boundaries
  // ==========================================================================
  group('block boundaries', () {
    test('lengths 0,55,56,63,64,127,128 all give 16-byte digests', () {
      for (final n in [0, 55, 56, 63, 64, 127, 128]) {
        expect(sumMd5(List<int>.filled(n, 0)).length, equals(16));
      }
    });

    test('55 and 56 bytes differ (padding crosses a block)', () {
      expect(sumMd5(List<int>.filled(55, 0)),
          isNot(equals(sumMd5(List<int>.filled(56, 0)))));
    });

    test('all seven boundary sizes are distinct', () {
      final seen = <String>{};
      for (final n in [0, 55, 56, 63, 64, 127, 128]) {
        seen.add(hexString(List<int>.filled(n, 0)));
      }
      expect(seen.length, equals(7));
    });
  });

  // ==========================================================================
  // Streaming hasher
  // ==========================================================================
  group('Md5Digest', () {
    test('single write matches one-shot', () {
      final h = Md5Digest()..update(a('abc'));
      expect(h.sumMd5(), equals(sumMd5(a('abc'))));
    });

    test('split mid-message matches one-shot', () {
      final h = Md5Digest()
        ..update(a('ab'))
        ..update(a('c'));
      expect(h.sumMd5(), equals(sumMd5(a('abc'))));
    });

    test('split on a block boundary matches one-shot', () {
      final data = List<int>.filled(128, 0);
      final h = Md5Digest()
        ..update(data.sublist(0, 64))
        ..update(data.sublist(64));
      expect(h.sumMd5(), equals(sumMd5(data)));
    });

    test('byte-at-a-time matches one-shot', () {
      final data = List<int>.generate(100, (i) => i);
      final h = Md5Digest();
      for (final b in data) {
        h.update([b]);
      }
      expect(h.sumMd5(), equals(sumMd5(data)));
    });

    test('empty stream matches empty one-shot', () {
      expect(Md5Digest().sumMd5(), equals(sumMd5(a(''))));
    });

    test('sumMd5() is non-destructive and can continue', () {
      final h = Md5Digest()..update(a('hello'));
      final d1 = h.sumMd5();
      expect(d1, equals(h.sumMd5()));
      h.update(a(' world'));
      expect(h.sumMd5(), equals(sumMd5(a('hello world'))));
      expect(d1, equals(sumMd5(a('hello'))));
    });

    test('hexDigest matches hexString', () {
      final h = Md5Digest()..update(a('abc'));
      expect(h.hexDigest(), equals(hexString(a('abc'))));
    });

    test('cloneDigest produces an independent copy', () {
      final h = Md5Digest()..update(a('ab'));
      final h2 = h.cloneDigest();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.sumMd5(), equals(sumMd5(a('abc'))));
      expect(h.sumMd5(), equals(sumMd5(a('abx'))));
    });

    test('one million "a" streamed in two halves', () {
      final data = List<int>.filled(1000000, 0x61);
      final h = Md5Digest()
        ..update(data.sublist(0, 500000))
        ..update(data.sublist(500000));
      expect(h.sumMd5(), equals(sumMd5(data)));
    });
  });
}
