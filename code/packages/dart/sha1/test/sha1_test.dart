import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_sha1/coding_adventures_sha1.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

void main() {
  // ==========================================================================
  // FIPS 180-4 / RFC 3174 known-answer vectors
  // ==========================================================================
  group('known-answer vectors', () {
    test('empty string', () {
      expect(hexString(a('')),
          equals('da39a3ee5e6b4b0d3255bfef95601890afd80709'));
    });
    test('"abc"', () {
      expect(hexString(a('abc')),
          equals('a9993e364706816aba3e25717850c26c9cd0d89d'));
    });
    test('448-bit message (56 bytes)', () {
      const msg = 'abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq';
      expect(msg.length, equals(56));
      expect(hexString(a(msg)),
          equals('84983e441c3bd26ebaae4aa1f95129e5e54670f1'));
    });
    test('one million "a"', () {
      final data = List<int>.filled(1000000, 0x61);
      expect(hexString(data),
          equals('34aa973cd4c4daa4f61eeb2bdbad27316534016f'));
    });
  });

  // ==========================================================================
  // Output format
  // ==========================================================================
  group('output format', () {
    test('digest is always 20 bytes', () {
      expect(sum1(a('')).length, equals(20));
      expect(sum1(a('hello world')).length, equals(20));
      expect(sum1(List<int>.filled(1000, 0)).length, equals(20));
    });
    test('digest is a Uint8List', () {
      expect(sum1(a('abc')), isA<Uint8List>());
    });
    test('hex string is 40 lowercase hex chars', () {
      final h = hexString(a('abc'));
      expect(h.length, equals(40));
      expect(RegExp(r'^[0-9a-f]{40}$').hasMatch(h), isTrue);
    });
  });

  // ==========================================================================
  // Core properties
  // ==========================================================================
  group('properties', () {
    test('deterministic', () {
      expect(sum1(a('hello')), equals(sum1(a('hello'))));
    });
    test('avalanche: one-char change flips many bits', () {
      final h1 = sum1(a('hello'));
      final h2 = sum1(a('helo'));
      expect(h1, isNot(equals(h2)));
      var bits = 0;
      for (var i = 0; i < 20; i++) {
        var x = h1[i] ^ h2[i];
        while (x != 0) {
          bits += x & 1;
          x >>= 1;
        }
      }
      expect(bits, greaterThan(30), reason: 'only $bits bits differed');
    });
    test('a null byte differs from the empty message', () {
      expect(sum1([0x00]), isNot(equals(sum1(a('')))));
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
    test('lengths 0,55,56,63,64,127,128 all give 20-byte digests', () {
      for (final n in [0, 55, 56, 63, 64, 127, 128]) {
        expect(sum1(List<int>.filled(n, 0)).length, equals(20));
      }
    });
    test('55 and 56 bytes differ (padding crosses a block)', () {
      expect(sum1(List<int>.filled(55, 0)),
          isNot(equals(sum1(List<int>.filled(56, 0)))));
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
  group('Sha1Digest', () {
    test('single write matches one-shot', () {
      final h = Sha1Digest()..update(a('abc'));
      expect(h.sum1(), equals(sum1(a('abc'))));
    });
    test('split mid-message matches one-shot', () {
      final h = Sha1Digest()
        ..update(a('ab'))
        ..update(a('c'));
      expect(h.sum1(), equals(sum1(a('abc'))));
    });
    test('split on a block boundary matches one-shot', () {
      final data = List<int>.filled(128, 0);
      final h = Sha1Digest()
        ..update(data.sublist(0, 64))
        ..update(data.sublist(64));
      expect(h.sum1(), equals(sum1(data)));
    });
    test('byte-at-a-time matches one-shot', () {
      final data = List<int>.generate(100, (i) => i);
      final h = Sha1Digest();
      for (final b in data) {
        h.update([b]);
      }
      expect(h.sum1(), equals(sum1(data)));
    });
    test('empty stream matches empty one-shot', () {
      expect(Sha1Digest().sum1(), equals(sum1(a(''))));
    });
    test('sum1() is non-destructive and can continue', () {
      final h = Sha1Digest()..update(a('hello'));
      final d1 = h.sum1();
      expect(d1, equals(h.sum1()));
      h.update(a(' world'));
      expect(h.sum1(), equals(sum1(a('hello world'))));
      expect(d1, equals(sum1(a('hello'))));
    });
    test('hexDigest matches hexString', () {
      final h = Sha1Digest()..update(a('abc'));
      expect(h.hexDigest(), equals(hexString(a('abc'))));
    });
    test('cloneDigest produces an independent copy', () {
      final h = Sha1Digest()..update(a('ab'));
      final h2 = h.cloneDigest();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.sum1(), equals(sum1(a('abc'))));
      expect(h.sum1(), equals(sum1(a('abx'))));
    });
    test('one million "a" streamed in two halves', () {
      final data = List<int>.filled(1000000, 0x61);
      final h = Sha1Digest()
        ..update(data.sublist(0, 500000))
        ..update(data.sublist(500000));
      expect(h.hexDigest(),
          equals('34aa973cd4c4daa4f61eeb2bdbad27316534016f'));
    });
  });
}
