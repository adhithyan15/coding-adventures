import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_sha256_native/coding_adventures_sha256_native.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

/// These tests exercise the *Rust* SHA-256 through dart:ffi, asserting the same
/// FIPS 180-4 answers and streaming behaviour as the pure-Dart port.
void main() {
  group('one-shot (native)', () {
    test('FIPS 180-4 vectors', () {
      expect(sha256Hex(a('')),
          equals('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'));
      expect(sha256Hex(a('abc')),
          equals('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'));
      expect(
          sha256Hex(a('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq')),
          equals('248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1'));
    });

    test('one million "a"', () {
      final data = List<int>.filled(1000000, 0x61);
      expect(sha256Hex(data),
          equals('cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0'));
    });

    test('digest is a 32-byte Uint8List', () {
      final d = sha256(a('abc'));
      expect(d, isA<Uint8List>());
      expect(d.length, equals(32));
    });

    test('block-boundary sizes are all distinct 32-byte digests', () {
      final seen = <String>{};
      for (final n in [55, 56, 63, 64, 127, 128]) {
        final d = sha256(List<int>.filled(n, 0));
        expect(d.length, equals(32));
        seen.add(sha256Hex(List<int>.filled(n, 0)));
      }
      expect(seen.length, equals(6));
    });

    test('null byte differs from empty', () {
      expect(sha256([0x00]), isNot(equals(sha256(a('')))));
    });
  });

  group('Sha256Hasher (native)', () {
    test('single write matches one-shot', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.digest(), equals(sha256(a('abc'))));
      h.dispose();
    });

    test('split mid-message and on a block boundary match one-shot', () {
      final h1 = Sha256Hasher()
        ..update(a('ab'))
        ..update(a('c'));
      expect(h1.digest(), equals(sha256(a('abc'))));

      final data = List<int>.filled(128, 0);
      final h2 = Sha256Hasher()
        ..update(data.sublist(0, 64))
        ..update(data.sublist(64));
      expect(h2.digest(), equals(sha256(data)));
    });

    test('byte-at-a-time matches one-shot', () {
      final data = List<int>.generate(100, (i) => i);
      final h = Sha256Hasher();
      for (final b in data) {
        h.update([b]);
      }
      expect(h.digest(), equals(sha256(data)));
    });

    test('empty stream matches empty one-shot', () {
      expect(Sha256Hasher().digest(), equals(sha256(a(''))));
    });

    test('digest() is non-destructive and can continue', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.digest(), equals(h.digest()));
      h.update(a('d'));
      expect(h.digest(), equals(sha256(a('abcd'))));
    });

    test('hexDigest matches sha256Hex', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.hexDigest(), equals(sha256Hex(a('abc'))));
    });

    test('cloneHasher produces an independent native handle', () {
      final h = Sha256Hasher()..update(a('ab'));
      final h2 = h.cloneHasher();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.digest(), equals(sha256(a('abc'))));
      expect(h.digest(), equals(sha256(a('abx'))));
      h.dispose();
      h2.dispose();
    });

    test('using a disposed hasher throws', () {
      final h = Sha256Hasher()..update(a('abc'));
      h.dispose();
      expect(() => h.update(a('x')), throwsStateError);
      expect(() => h.digest(), throwsStateError);
      h.dispose(); // idempotent
    });

    test('one million "a" streamed in two halves', () {
      final data = List<int>.filled(1000000, 0x61);
      final h = Sha256Hasher()
        ..update(data.sublist(0, 500000))
        ..update(data.sublist(500000));
      expect(h.hexDigest(),
          equals('cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0'));
    });
  });
}
