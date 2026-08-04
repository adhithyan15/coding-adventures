import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';
import 'package:test/test.dart';

List<int> a(String s) => utf8.encode(s);

void main() {
  // ==========================================================================
  // FIPS 180-4 known-answer test vectors
  // ==========================================================================
  group('FIPS 180-4 vectors', () {
    test('empty string', () {
      expect(sha256Hex(a('')),
          equals('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'));
    });

    test('"abc"', () {
      expect(sha256Hex(a('abc')),
          equals('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'));
    });

    test('448-bit message (56 bytes)', () {
      const msg = 'abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq';
      expect(msg.length, equals(56));
      expect(sha256Hex(a(msg)),
          equals('248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1'));
    });

    test('one million "a"', () {
      final data = List<int>.filled(1000000, 0x61); // 'a'
      expect(sha256Hex(data),
          equals('cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0'));
    });
  });

  // ==========================================================================
  // Output format
  // ==========================================================================
  group('output format', () {
    test('digest is always 32 bytes', () {
      expect(sha256(a('')).length, equals(32));
      expect(sha256(a('hello world')).length, equals(32));
      expect(sha256(List<int>.filled(1000, 0)).length, equals(32));
    });

    test('digest is a Uint8List of byte values', () {
      final d = sha256(a('abc'));
      expect(d, isA<Uint8List>());
      expect(d.every((b) => b >= 0 && b <= 255), isTrue);
    });

    test('hex string is 64 lowercase hex chars', () {
      final h = sha256Hex(a('abc'));
      expect(h.length, equals(64));
      expect(RegExp(r'^[0-9a-f]{64}$').hasMatch(h), isTrue);
    });
  });

  // ==========================================================================
  // Core properties
  // ==========================================================================
  group('properties', () {
    test('deterministic', () {
      expect(sha256(a('hello')), equals(sha256(a('hello'))));
    });

    test('avalanche: one-char change flips many bits', () {
      final h1 = sha256(a('hello'));
      final h2 = sha256(a('helo'));
      expect(h1, isNot(equals(h2)));
      var bits = 0;
      for (var i = 0; i < 32; i++) {
        var x = h1[i] ^ h2[i];
        while (x != 0) {
          bits += x & 1;
          x >>= 1;
        }
      }
      expect(bits, greaterThan(40), reason: 'only $bits bits differed');
    });

    test('a null byte differs from the empty message', () {
      expect(sha256([0x00]), isNot(equals(sha256(a('')))));
    });

    test('every single byte value hashes distinctly', () {
      final seen = <String>{};
      for (var i = 0; i <= 255; i++) {
        seen.add(sha256Hex([i]));
      }
      expect(seen.length, equals(256));
    });
  });

  // ==========================================================================
  // Padding / block boundaries (the classic off-by-one traps)
  // ==========================================================================
  group('block boundaries', () {
    test('lengths 55, 56, 63, 64, 127, 128 all produce 32-byte digests', () {
      for (final n in [55, 56, 63, 64, 127, 128]) {
        expect(sha256(List<int>.filled(n, 0)).length, equals(32));
      }
    });

    test('55 and 56 bytes differ (padding crosses a block)', () {
      expect(sha256(List<int>.filled(55, 0)),
          isNot(equals(sha256(List<int>.filled(56, 0)))));
    });

    test('all six boundary sizes are distinct', () {
      final seen = <String>{};
      for (final n in [55, 56, 63, 64, 127, 128]) {
        seen.add(sha256Hex(List<int>.filled(n, 0)));
      }
      expect(seen.length, equals(6));
    });
  });

  // ==========================================================================
  // Streaming hasher
  // ==========================================================================
  group('Sha256Hasher', () {
    test('single write matches one-shot', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.digest(), equals(sha256(a('abc'))));
    });

    test('split mid-message matches one-shot', () {
      final h = Sha256Hasher()
        ..update(a('ab'))
        ..update(a('c'));
      expect(h.digest(), equals(sha256(a('abc'))));
    });

    test('split on a block boundary matches one-shot', () {
      final data = List<int>.filled(128, 0);
      final h = Sha256Hasher()
        ..update(data.sublist(0, 64))
        ..update(data.sublist(64));
      expect(h.digest(), equals(sha256(data)));
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

    test('digest() is non-destructive and repeatable', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.digest(), equals(h.digest()));
      // Can keep updating after a digest() call.
      h.update(a('d'));
      expect(h.digest(), equals(sha256(a('abcd'))));
    });

    test('hexDigest matches sha256Hex', () {
      final h = Sha256Hasher()..update(a('abc'));
      expect(h.hexDigest(), equals(sha256Hex(a('abc'))));
    });

    test('cloneHasher produces an independent copy', () {
      final h = Sha256Hasher()..update(a('ab'));
      final h2 = h.cloneHasher();
      h2.update(a('c'));
      h.update(a('x'));
      expect(h2.digest(), equals(sha256(a('abc'))));
      expect(h.digest(), equals(sha256(a('abx'))));
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
