import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_md5_native/coding_adventures_md5_native.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

/// Exercises the Rust MD5 through dart:ffi, asserting the same RFC 1321 answers
/// and streaming behaviour as the pure-Dart port.
void main() {
  group('one-shot (native)', () {
    test('RFC 1321 vectors', () {
      expect(hexString(a('')), equals('d41d8cd98f00b204e9800998ecf8427e'));
      expect(hexString(a('a')), equals('0cc175b9c0f1b6a831c399e269772661'));
      expect(hexString(a('abc')), equals('900150983cd24fb0d6963f7d28e17f72'));
      expect(hexString(a('message digest')),
          equals('f96b697d7cb7938d525a2f31aaf161d0'));
      expect(hexString(a('abcdefghijklmnopqrstuvwxyz')),
          equals('c3fcd3d76192e4007dfb496cca67e13b'));
    });

    test('bytes 0x00..0xFF known digest (little-endian check)', () {
      final data = List<int>.generate(256, (i) => i);
      expect(hexString(data), equals('e2c865db4162bed963bfaa9ef6ac18f0'));
    });

    test('digest is a 16-byte Uint8List', () {
      final d = sumMd5(a('abc'));
      expect(d, isA<Uint8List>());
      expect(d.length, equals(16));
    });

    test('block-boundary sizes are all distinct 16-byte digests', () {
      final seen = <String>{};
      for (final n in [0, 55, 56, 63, 64, 127, 128]) {
        expect(sumMd5(List<int>.filled(n, 0)).length, equals(16));
        seen.add(hexString(List<int>.filled(n, 0)));
      }
      expect(seen.length, equals(7));
    });

    test('null byte differs from empty', () {
      expect(sumMd5([0x00]), isNot(equals(sumMd5(a('')))));
    });
  });

  group('Md5Digest (native)', () {
    test('single write matches one-shot', () {
      final h = Md5Digest()..update(a('abc'));
      expect(h.sumMd5(), equals(sumMd5(a('abc'))));
      h.dispose();
    });

    test('split mid-message and on a block boundary match one-shot', () {
      final h1 = Md5Digest()
        ..update(a('ab'))
        ..update(a('c'));
      expect(h1.sumMd5(), equals(sumMd5(a('abc'))));

      final data = List<int>.filled(128, 0);
      final h2 = Md5Digest()
        ..update(data.sublist(0, 64))
        ..update(data.sublist(64));
      expect(h2.sumMd5(), equals(sumMd5(data)));
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
      expect(h.sumMd5(), equals(h.sumMd5()));
      h.update(a(' world'));
      expect(h.sumMd5(), equals(sumMd5(a('hello world'))));
    });

    test('hexDigest matches hexString', () {
      final h = Md5Digest()..update(a('abc'));
      expect(h.hexDigest(), equals(hexString(a('abc'))));
    });

    test('cloneDigest produces an independent native handle', () {
      final h = Md5Digest()..update(a('ab'));
      final h2 = h.cloneDigest();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.sumMd5(), equals(sumMd5(a('abc'))));
      expect(h.sumMd5(), equals(sumMd5(a('abx'))));
      h.dispose();
      h2.dispose();
    });

    test('using a disposed digest throws', () {
      final h = Md5Digest()..update(a('abc'));
      h.dispose();
      expect(() => h.update(a('x')), throwsStateError);
      expect(() => h.sumMd5(), throwsStateError);
      h.dispose(); // idempotent
    });

    test('one million "a" streamed in two halves', () {
      final data = List<int>.filled(1000000, 0x61);
      final h = Md5Digest()
        ..update(data.sublist(0, 500000))
        ..update(data.sublist(500000));
      expect(h.hexDigest(), equals(sumMd5(data).map((b) => b.toRadixString(16).padLeft(2, '0')).join()));
    });
  });
}
