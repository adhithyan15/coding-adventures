// Unit tests for the SHA-384 lane: all 9 reference tests mirrored
// case-for-case (FIPS vectors, streaming, transactional failure,
// width), plus lane extras (dispose semantics, hex helper, error
// value).

import 'dart:typed_data';

import 'package:coding_adventures_sha384/sha384.dart';
import 'package:test/test.dart';

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('sha384', () {
    test('fips empty message', () {
      expect(
        _hex(sha384([])),
        equals(
          '38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da'
          '274edebfe76f65fbd51ad2f14898b95b',
        ),
      );
    });

    test('fips abc', () {
      expect(
        _hex(sha384('abc'.codeUnits)),
        equals(
          'cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed'
          '8086072ba1e7cc2358baeca134c825a7',
        ),
      );
    });

    test('fips 896-bit message crosses padding boundary', () {
      final message =
          'abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno'
                  'ijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu'
              .codeUnits;
      expect(message.length, equals(112));
      expect(
        _hex(sha384(message)),
        equals(
          '09330c33f71147e83d192fc782cd1b4753111b173b3b05d22fa08086e3b0f712'
          'fcc7c71a557e2db966c3e9fa91746039',
        ),
      );
    });

    test('fips million a streams without allocating the message', () {
      final chunk = Uint8List.fromList(List<int>.filled(1000, 0x61));
      final hasher = Sha384Hasher();
      for (var i = 0; i < 1000; i++) {
        hasher.update(chunk);
      }
      expect(
        _hex(hasher.digest()),
        equals(
          '9d0e1809716474cb086e834e310a4a1ced149e9c00f248527972cec5704c2a5b'
          '07b8b3dc38ecc4ebae97ddd87f3d8985',
        ),
      );
    });

    test('every relevant partition matches one-shot', () {
      final message =
          Uint8List.fromList([for (var i = 0; i < 1037; i++) i & 0xff]);
      final expected = sha384(message);
      for (final chunkSize in [
        1,
        2,
        3,
        7,
        13,
        64,
        111,
        112,
        127,
        128,
        129,
        255
      ]) {
        final hasher = Sha384Hasher();
        for (var offset = 0; offset < message.length; offset += chunkSize) {
          final end = (offset + chunkSize).clamp(0, message.length);
          hasher.update(message.sublist(offset, end));
        }
        expect(_hex(hasher.digest()), equals(_hex(expected)),
            reason: 'chunk size $chunkSize');
      }
    });

    test('digest is non-destructive', () {
      final hasher = Sha384Hasher();
      hasher.update('ab'.codeUnits);
      final prefix = hasher.digest();
      expect(_hex(prefix), equals(_hex(sha384('ab'.codeUnits))));
      expect(_hex(hasher.digest()), equals(_hex(prefix)));
      hasher.update('c'.codeUnits);
      expect(_hex(hasher.digest()), equals(_hex(sha384('abc'.codeUnits))));
    });

    test('empty updates do not change the digest', () {
      final hasher = Sha384Hasher();
      hasher.update('abc'.codeUnits);
      final before = hasher.digest();
      hasher.update([]);
      expect(_hex(hasher.digest()), equals(_hex(before)));
    });

    test('length failure is payload-free and transactional', () {
      final hasher = Sha384Hasher();
      hasher.update('abc'.codeUnits);
      final maxBytes = ((BigInt.one << 128) - BigInt.one) >> 3;
      setSha384ByteCounterForTesting(hasher, maxBytes);
      final digestBefore = _hex(hasher.digest());

      expect(
        () => hasher.update('secret'.codeUnits),
        throwsA(equals(const Sha384MessageTooLong())),
      );
      // State untouched: same digest, same counter behavior.
      expect(_hex(hasher.digest()), equals(digestBefore));
      expect(
        () => hasher.update([1]),
        throwsA(isA<Sha384MessageTooLong>()),
      );
      expect(
        const Sha384MessageTooLong().toString(),
        equals('SHA-384 message is too long'),
      );
    });

    test('digest has exact sha384 width', () {
      expect(sha384('width'.codeUnits).length, equals(48));
      expect(sha384Hex('width'.codeUnits).length, equals(96));
    });

    // --- Lane extras ---

    test('dispose scrubs and locks the hasher', () {
      final hasher = Sha384Hasher();
      hasher.update('abc'.codeUnits);
      expect(hasher.isDisposed, isFalse);
      hasher.dispose();
      expect(hasher.isDisposed, isTrue);
      // Second dispose is safe.
      hasher.dispose();
      // Any use after dispose fails loudly instead of hashing zeros.
      expect(() => hasher.update([1]), throwsStateError);
      expect(() => hasher.digest(), throwsStateError);
    });

    test('one-shot helpers agree with streaming', () {
      final data = [for (var i = 0; i < 300; i++) (i * 13 + 5) & 0xff];
      final hasher = Sha384Hasher()
        ..update(data.sublist(0, 100))
        ..update(data.sublist(100));
      expect(_hex(sha384(data)), equals(_hex(hasher.digest())));
      expect(sha384Hex(data), equals(_hex(sha384(data))));
    });

    test('error value semantics', () {
      expect(
        const Sha384MessageTooLong(),
        equals(const Sha384MessageTooLong()),
      );
      expect(
        const Sha384MessageTooLong().hashCode,
        equals(const Sha384MessageTooLong().hashCode),
      );
      expect(
        const Sha384MessageTooLong(),
        isNot(equals(StateError('x'))),
      );
    });
  });
}
