/// Unit tests for `coding_adventures_aes`: all 38 reference tests
/// mirrored case-for-case (FIPS vectors, fixed-path agreement,
/// S-box properties, key schedule, round-trips, avalanche, errors),
/// plus lane extras (block-length checks, fixed-path lengths, error
/// values). The circuit-vs-table test imports `src/sbox.dart`
/// directly, following the x25519 lane.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart';
import 'package:coding_adventures_aes/src/sbox.dart' as sboxlib;
import 'package:test/test.dart';

Uint8List _h(String hex) {
  final clean = hex.replaceAll(' ', '');
  final out = Uint8List(clean.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(clean.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('FIPS vectors', () {
    test('aes128 appendix B encrypt', () {
      final key = _h('2b7e151628aed2a6abf7158809cf4f3c');
      final plain = _h('3243f6a8885a308d313198a2e0370734');
      expect(
        _hex(encryptBlock(plain, key)),
        equals('3925841d02dc09fbdc118597196a0b32'),
      );
    });

    test('aes128 appendix B decrypt', () {
      final key = _h('2b7e151628aed2a6abf7158809cf4f3c');
      final ct = _h('3925841d02dc09fbdc118597196a0b32');
      expect(
        _hex(decryptBlock(ct, key)),
        equals('3243f6a8885a308d313198a2e0370734'),
      );
    });

    test('aes128 appendix C1', () {
      final key = _h('000102030405060708090a0b0c0d0e0f');
      final plain = _h('00112233445566778899aabbccddeeff');
      expect(
        _hex(encryptBlock(plain, key)),
        equals('69c4e0d86a7b0430d8cdb78070b4c55a'),
      );
    });

    test('aes128 fixed matches C1 vector', () {
      final key = _h('000102030405060708090a0b0c0d0e0f');
      final plain = _h('00112233445566778899aabbccddeeff');
      final output = Uint8List(16);
      aes128EncryptBlockFixed(plain, key, output);
      expect(
        _hex(output),
        equals('69c4e0d86a7b0430d8cdb78070b4c55a'),
      );
    });

    test('fixed sbox matches all legacy table entries', () {
      final sb = sbox();
      for (var input = 0; input < 256; input++) {
        expect(sboxlib.sboxFixed(input), equals(sb[input]),
            reason: 'input $input');
      }
    });

    test('fixed matches legacy across varied inputs', () {
      for (var seed = 0; seed < 64; seed++) {
        final key = Uint8List.fromList(
            List<int>.generate(16, (i) => (seed * 17 + i * 29) & 0xff));
        final plain = Uint8List.fromList(List<int>.generate(16, (i) {
          final rotated =
              ((seed << (i % 8)) | (seed >> ((8 - i % 8) % 8))) & 0xff;
          return (rotated + i * 11) & 0xff;
        }));
        final fixed = Uint8List(16);
        aes128EncryptBlockFixed(plain, key, fixed);
        expect(fixed, equals(encryptBlock(plain, key)), reason: 'seed $seed');
      }
    });

    test('aes192 appendix C2 encrypt', () {
      final key = _h('000102030405060708090a0b0c0d0e0f1011121314151617');
      final plain = _h('00112233445566778899aabbccddeeff');
      expect(
        _hex(encryptBlock(plain, key)),
        equals('dda97ca4864cdfe06eaf70a0ec0d7191'),
      );
    });

    test('aes192 appendix C2 decrypt', () {
      final key = _h('000102030405060708090a0b0c0d0e0f1011121314151617');
      final ct = _h('dda97ca4864cdfe06eaf70a0ec0d7191');
      expect(
        _hex(decryptBlock(ct, key)),
        equals('00112233445566778899aabbccddeeff'),
      );
    });

    test('aes256 vector encrypt', () {
      final key = _h(
          '603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4');
      final plain = _h('6bc1bee22e409f96e93d7e117393172a');
      expect(
        _hex(encryptBlock(plain, key)),
        equals('f3eed1bdb5d2a03c064b5a7e3db181f8'),
      );
    });

    test('aes256 vector decrypt', () {
      final key = _h(
          '603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4');
      final ct = _h('f3eed1bdb5d2a03c064b5a7e3db181f8');
      expect(
        _hex(decryptBlock(ct, key)),
        equals('6bc1bee22e409f96e93d7e117393172a'),
      );
    });

    test('aes256 appendix C3', () {
      final key = _h(
          '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f');
      final plain = _h('00112233445566778899aabbccddeeff');
      final ct = _h('8ea2b7ca516745bfeafc49904b496089');
      expect(encryptBlock(plain, key), equals(ct));
      expect(decryptBlock(ct, key), equals(plain));
    });
  });

  group('sbox properties', () {
    test('sbox length', () {
      expect(sbox().length, equals(256));
    });

    test('inv sbox length', () {
      expect(invSbox().length, equals(256));
    });

    test('sbox is bijection', () {
      final seen = List<bool>.filled(256, false);
      for (final v in sbox()) {
        seen[v] = true;
      }
      expect(seen.every((s) => s), isTrue);
    });

    test('inv sbox is bijection', () {
      final seen = List<bool>.filled(256, false);
      for (final v in invSbox()) {
        seen[v] = true;
      }
      expect(seen.every((s) => s), isTrue);
    });

    test('sbox inv sbox inverse', () {
      final sb = sbox();
      final isb = invSbox();
      for (var b = 0; b < 256; b++) {
        expect(isb[sb[b]], equals(b), reason: 'failed at b=$b');
      }
    });

    test('known values FIPS figure 7', () {
      final sb = sbox();
      expect(sb[0x00], equals(0x63));
      expect(sb[0x01], equals(0x7c));
      expect(sb[0xff], equals(0x16));
      expect(sb[0x53], equals(0xed));
    });

    test('inv sbox known values', () {
      final isb = invSbox();
      expect(isb[0x63], equals(0x00));
      expect(isb[0x7c], equals(0x01));
    });

    test('no fixed points', () {
      final sb = sbox();
      for (var b = 0; b < 256; b++) {
        expect(sb[b], isNot(equals(b)), reason: 'fixed point at $b');
      }
    });
  });

  group('key schedule', () {
    test('aes128 round count', () {
      final key = Uint8List.fromList(List<int>.generate(16, (i) => i));
      expect(expandKey(key).length, equals(11));
    });

    test('aes192 round count', () {
      final key = Uint8List.fromList(List<int>.generate(24, (i) => i));
      expect(expandKey(key).length, equals(13));
    });

    test('aes256 round count', () {
      final key = Uint8List.fromList(List<int>.generate(32, (i) => i));
      expect(expandKey(key).length, equals(15));
    });

    test('round key shape', () {
      for (final keyLen in [16, 24, 32]) {
        final key = Uint8List.fromList(List<int>.generate(keyLen, (i) => i));
        final rks = expandKey(key);
        for (final rk in rks) {
          // Flat column-major: 16 bytes = 4 rows × 4 columns.
          expect(rk.length, equals(16));
        }
      }
    });

    test('first round key equals key', () {
      final key = _h('2b7e151628aed2a6abf7158809cf4f3c');
      final rks = expandKey(key);
      // Flat column-major layout reproduces the key bytes exactly.
      expect(rks[0], equals(key));
    });

    test('different keys differ', () {
      final rks1 =
          expandKey(Uint8List.fromList(List<int>.generate(16, (i) => i)));
      final rks2 =
          expandKey(Uint8List.fromList(List<int>.generate(16, (i) => i + 1)));
      expect(rks1[0], isNot(equals(rks2[0])));
    });

    test('invalid length', () {
      for (final keyLen in [15, 17, 20]) {
        final key = Uint8List.fromList(List<int>.generate(keyLen, (i) => i));
        expect(
          () => expandKey(key),
          throwsA(equals(AesInvalidKeyLength(keyLen))),
          reason: 'key length $keyLen',
        );
      }
    });
  });

  group('round trips', () {
    test('aes128 fips vector', () {
      final key = _h('2b7e151628aed2a6abf7158809cf4f3c');
      final plain = _h('3243f6a8885a308d313198a2e0370734');
      final ct = encryptBlock(plain, key);
      expect(decryptBlock(ct, key), equals(plain));
    });

    test('aes192 sweep', () {
      final key = _h('000102030405060708090a0b0c0d0e0f1011121314151617');
      for (var start = 0; start <= 240; start += 16) {
        final plain = Uint8List.fromList(
            List<int>.generate(16, (i) => (start + i) & 0xff));
        final ct = encryptBlock(plain, key);
        expect(decryptBlock(ct, key), equals(plain), reason: 'start $start');
      }
    });

    test('aes256 sweep', () {
      final key = _h(
          '603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4');
      for (var start = 0; start <= 240; start += 16) {
        final plain = Uint8List.fromList(
            List<int>.generate(16, (i) => (start + i) & 0xff));
        final ct = encryptBlock(plain, key);
        expect(decryptBlock(ct, key), equals(plain), reason: 'start $start');
      }
    });

    test('all zeros', () {
      for (final keyLen in [16, 24, 32]) {
        final key = Uint8List(keyLen);
        const plain = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        final ct = encryptBlock(plain, key);
        expect(decryptBlock(ct, key), equals(plain));
      }
    });

    test('all ff', () {
      for (final keyLen in [16, 24, 32]) {
        final key = Uint8List.fromList(List<int>.filled(keyLen, 0xff));
        final plain = Uint8List.fromList(List<int>.filled(16, 0xff));
        final ct = encryptBlock(plain, key);
        expect(decryptBlock(ct, key), equals(plain));
      }
    });

    test('identity key and plain', () {
      for (final keyLen in [16, 24, 32]) {
        final key = Uint8List.fromList(List<int>.generate(keyLen, (i) => i));
        final plain = Uint8List.fromList(List<int>.generate(16, (i) => i));
        final ct = encryptBlock(plain, key);
        expect(decryptBlock(ct, key), equals(plain));
      }
    });

    test('avalanche effect', () {
      final key = Uint8List.fromList(List<int>.generate(16, (i) => i));
      final plain1 = Uint8List(16);
      final plain2 = Uint8List(16)..[0] = 0x01;
      final ct1 = encryptBlock(plain1, key);
      final ct2 = encryptBlock(plain2, key);
      var diffBits = 0;
      for (var i = 0; i < 16; i++) {
        var x = ct1[i] ^ ct2[i];
        while (x != 0) {
          diffBits += x & 1;
          x >>= 1;
        }
      }
      expect(diffBits, greaterThan(32),
          reason: 'only $diffBits bits differ — poor diffusion');
    });
  });

  group('errors', () {
    test('encrypt wrong key length', () {
      final plain = Uint8List(16);
      for (final keyLen in [10, 20]) {
        final key = Uint8List.fromList(List<int>.generate(keyLen, (i) => i));
        expect(
          () => encryptBlock(plain, key),
          throwsA(equals(AesInvalidKeyLength(keyLen))),
          reason: 'key length $keyLen',
        );
      }
    });

    test('decrypt wrong key length', () {
      final ct = Uint8List(16);
      for (final keyLen in [15, 17]) {
        final key = Uint8List.fromList(List<int>.generate(keyLen, (i) => i));
        expect(
          () => decryptBlock(ct, key),
          throwsA(equals(AesInvalidKeyLength(keyLen))),
          reason: 'key length $keyLen',
        );
      }
    });
  });

  group('edge cases', () {
    test('encrypt is deterministic', () {
      final key = Uint8List.fromList(List<int>.generate(16, (i) => i));
      final plain = Uint8List.fromList(List<int>.filled(16, 0xab));
      expect(encryptBlock(plain, key), equals(encryptBlock(plain, key)));
    });

    test('different keys different ciphertext', () {
      final plain = Uint8List(16);
      final key1 = Uint8List.fromList(List<int>.generate(16, (i) => i));
      final key2 = Uint8List.fromList(List<int>.generate(16, (i) => i + 1));
      expect(
          encryptBlock(plain, key1), isNot(equals(encryptBlock(plain, key2))));
    });

    test('different plaintexts different ciphertext', () {
      final key = Uint8List.fromList(List<int>.generate(16, (i) => i));
      final ct1 = encryptBlock(Uint8List(16), key);
      final ct2 =
          encryptBlock(Uint8List.fromList(List<int>.filled(16, 1)), key);
      expect(ct1, isNot(equals(ct2)));
    });
  });

  group('lane extras', () {
    test('block lengths are checked', () {
      final key = Uint8List.fromList(List<int>.generate(16, (i) => i));
      expect(() => encryptBlock(Uint8List(15), key), throwsArgumentError);
      expect(() => encryptBlock(Uint8List(17), key), throwsArgumentError);
      expect(() => decryptBlock(Uint8List(15), key), throwsArgumentError);
      expect(() => decryptBlock(Uint8List(17), key), throwsArgumentError);
    });

    test('fixed path lengths are checked', () {
      final good = Uint8List(16);
      expect(() => aes128EncryptBlockFixed(Uint8List(15), good, good),
          throwsArgumentError);
      expect(() => aes128EncryptBlockFixed(good, Uint8List(17), good),
          throwsArgumentError);
      expect(() => aes128EncryptBlockFixed(good, good, Uint8List(0)),
          throwsArgumentError);
    });

    test('error values compare by payload', () {
      expect(
          const AesInvalidKeyLength(10), equals(const AesInvalidKeyLength(10)));
      expect(const AesInvalidKeyLength(10),
          isNot(equals(const AesInvalidKeyLength(20))));
      expect(const AesInvalidKeyLength(10).hashCode,
          equals(const AesInvalidKeyLength(10).hashCode));
      expect(const AesInvalidKeyLength(10).toString(),
          equals('AES key must be 16, 24, or 32 bytes; got 10'));
    });
  });
}
