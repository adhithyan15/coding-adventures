import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_sha1_native/coding_adventures_sha1_native.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

/// Exercises the Rust SHA-1 through dart:ffi, asserting the same FIPS/RFC answers
/// and streaming behaviour as the pure-Dart port.
void main() {
  group('one-shot (native)', () {
    test('known-answer vectors', () {
      expect(hexString(a('')), equals('da39a3ee5e6b4b0d3255bfef95601890afd80709'));
      expect(hexString(a('abc')), equals('a9993e364706816aba3e25717850c26c9cd0d89d'));
      expect(
          hexString(a('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq')),
          equals('84983e441c3bd26ebaae4aa1f95129e5e54670f1'));
    });
    test('one million "a"', () {
      final data = List<int>.filled(1000000, 0x61);
      expect(hexString(data), equals('34aa973cd4c4daa4f61eeb2bdbad27316534016f'));
    });
    test('digest is a 20-byte Uint8List', () {
      final d = sum1(a('abc'));
      expect(d, isA<Uint8List>());
      expect(d.length, equals(20));
    });
    test('block-boundary sizes are all distinct 20-byte digests', () {
      final seen = <String>{};
      for (final n in [0, 55, 56, 63, 64, 127, 128]) {
        expect(sum1(List<int>.filled(n, 0)).length, equals(20));
        seen.add(hexString(List<int>.filled(n, 0)));
      }
      expect(seen.length, equals(7));
    });
    test('null byte differs from empty', () {
      expect(sum1([0x00]), isNot(equals(sum1(a('')))));
    });
  });

  group('Sha1Digest (native)', () {
    test('single write matches one-shot', () {
      final h = Sha1Digest()..update(a('abc'));
      expect(h.sum1(), equals(sum1(a('abc'))));
      h.dispose();
    });
    test('split mid-message and on a block boundary match one-shot', () {
      final h1 = Sha1Digest()..update(a('ab'))..update(a('c'));
      expect(h1.sum1(), equals(sum1(a('abc'))));
      final data = List<int>.filled(128, 0);
      final h2 = Sha1Digest()..update(data.sublist(0, 64))..update(data.sublist(64));
      expect(h2.sum1(), equals(sum1(data)));
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
      expect(h.sum1(), equals(h.sum1()));
      h.update(a(' world'));
      expect(h.sum1(), equals(sum1(a('hello world'))));
    });
    test('hexDigest matches hexString', () {
      final h = Sha1Digest()..update(a('abc'));
      expect(h.hexDigest(), equals(hexString(a('abc'))));
    });
    test('cloneDigest produces an independent native handle', () {
      final h = Sha1Digest()..update(a('ab'));
      final h2 = h.cloneDigest();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.sum1(), equals(sum1(a('abc'))));
      expect(h.sum1(), equals(sum1(a('abx'))));
      h.dispose();
      h2.dispose();
    });
    test('using a disposed digest throws', () {
      final h = Sha1Digest()..update(a('abc'));
      h.dispose();
      expect(() => h.update(a('x')), throwsStateError);
      expect(() => h.sum1(), throwsStateError);
      h.dispose();
    });
  });
}
