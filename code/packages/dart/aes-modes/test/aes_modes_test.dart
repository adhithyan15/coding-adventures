/// Unit tests for `coding_adventures_aes_modes`: all 35 reference
/// tests mirrored case-for-case (PKCS#7, ECB/CBC/CTR vectors and
/// round-trips, GHASH incl. the branching reference model, fixed
/// GCM vectors and failure atomicity, compat GCM vectors and
/// tamper rejection), plus lane extras (fixed-buffer lengths, tag
/// length, error values). The length-exhaustion test imports the
/// public-in-source validator directly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes_modes/aes_modes.dart';
import 'package:coding_adventures_aes_modes/src/fixed.dart' as fixedlib;
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

/// Branching GHASH-multiply reference model (mirrors the test-only
/// model in the reference suite, not the mask-selected boundary).
Uint8List _ghashMultiplyReference(Uint8List x, Uint8List h) {
  final product = Uint8List(16);
  final multiplier = Uint8List.fromList(h);
  for (var bitIndex = 0; bitIndex < 128; bitIndex++) {
    if ((x[bitIndex ~/ 8] >> (7 - bitIndex % 8)) & 1 == 1) {
      for (var index = 0; index < 16; index++) {
        product[index] ^= multiplier[index];
      }
    }
    final carry = multiplier[15] & 1;
    for (var index = 15; index >= 1; index--) {
      multiplier[index] =
          (multiplier[index] >> 1) | ((multiplier[index - 1] & 1) << 7);
    }
    multiplier[0] >>= 1;
    if (carry == 1) {
      multiplier[0] ^= 0xe1;
    }
  }
  return product;
}

/// Branching GHASH reference model over AAD, ciphertext, and the
/// 128-bit big-endian length block.
Uint8List _ghashReference(Uint8List h, List<int> aad, List<int> ciphertext) {
  var accumulator = Uint8List(16);
  for (final data in [aad, ciphertext]) {
    for (var offset = 0; offset < data.length; offset += 16) {
      final end = offset + 16 < data.length ? offset + 16 : data.length;
      final block = Uint8List(16);
      block.setRange(0, end - offset, data.sublist(offset, end));
      for (var index = 0; index < 16; index++) {
        block[index] ^= accumulator[index];
      }
      accumulator = _ghashMultiplyReference(block, h);
    }
  }
  final lengthBlock = Uint8List(16);
  final aadBits = aad.length * 8;
  final ctBits = ciphertext.length * 8;
  for (var i = 0; i < 8; i++) {
    lengthBlock[i] = (aadBits >> (8 * (7 - i))) & 0xff;
    lengthBlock[8 + i] = (ctBits >> (8 * (7 - i))) & 0xff;
  }
  for (var index = 0; index < 16; index++) {
    lengthBlock[index] ^= accumulator[index];
  }
  return _ghashMultiplyReference(lengthBlock, h);
}

const _nistKey = '2b7e151628aed2a6abf7158809cf4f3c';
const _nistBlock1 = '6bc1bee22e409f96e93d7e117393172a';
const _nistAll = '6bc1bee22e409f96e93d7e117393172a'
    'ae2d8a571e03ac9c9eb76fac45af8e51'
    '30c81c46a35ce411e5fbc1191a0a52ef'
    'f69f2445df4f9b17ad2b417be66c3710';

void main() {
  group('pkcs7', () {
    test('pad aligned', () {
      final padded = pkcs7Pad(Uint8List.fromList(List.filled(16, 0xaa)));
      expect(padded.length, equals(32));
      for (final b in padded.sublist(16)) {
        expect(b, equals(16));
      }
    });

    test('pad 13 bytes', () {
      final padded = pkcs7Pad(Uint8List.fromList(List.filled(13, 0xbb)));
      expect(padded.length, equals(16));
      expect(padded[13], equals(3));
      expect(padded[14], equals(3));
      expect(padded[15], equals(3));
    });

    test('roundtrip', () {
      final input = Uint8List.fromList([1, 2, 3, 4, 5]);
      expect(pkcs7Unpad(pkcs7Pad(input)), equals(input));
    });

    test('rejects invalid', () {
      final bad = Uint8List(16)..[15] = 0;
      expect(() => pkcs7Unpad(bad), throwsA(isA<InvalidPadding>()));
      bad[15] = 2;
      bad[14] = 3;
      expect(() => pkcs7Unpad(bad), throwsA(isA<InvalidPadding>()));
      expect(
          () => pkcs7Unpad(Uint8List(15)), throwsA(isA<InvalidPaddedLength>()));
      expect(
          () => pkcs7Unpad(Uint8List(0)), throwsA(isA<InvalidPaddedLength>()));
    });
  });

  group('ecb', () {
    test('nist block1', () {
      final ct = ecbEncrypt(_unhex(_nistBlock1), _unhex(_nistKey));
      expect(
          _hex(ct.sublist(0, 16)), equals('3ad77bb40d7a3660a89ecaf32466ef97'));
    });

    test('nist all blocks', () {
      final ct = ecbEncrypt(_unhex(_nistAll), _unhex(_nistKey));
      expect(ct.length, equals(80));
      expect(
          _hex(ct.sublist(0, 16)), equals('3ad77bb40d7a3660a89ecaf32466ef97'));
      expect(
          _hex(ct.sublist(16, 32)), equals('f5d3d58503b9699de785895a96fdbaaf'));
      expect(
          _hex(ct.sublist(32, 48)), equals('43b1cd7f598ece23881b00e3ed030688'));
      expect(
          _hex(ct.sublist(48, 64)), equals('7b0c785e27e8ad3f8223207104725dd4'));
    });

    test('roundtrip', () {
      final key = _unhex(_nistKey);
      final pt = _unhex(_nistAll);
      expect(ecbDecrypt(ecbEncrypt(pt, key), key), equals(pt));
    });

    test('identical blocks produce identical ct', () {
      final key = _unhex(_nistKey);
      final block = _unhex(_nistBlock1);
      final two = Uint8List.fromList([...block, ...block]);
      final ct = ecbEncrypt(two, key);
      expect(_hex(ct.sublist(0, 16)), equals(_hex(ct.sublist(16, 32))));
    });
  });

  group('cbc', () {
    test('nist block1', () {
      final ct = cbcEncrypt(_unhex(_nistBlock1), _unhex(_nistKey),
          _unhex('000102030405060708090a0b0c0d0e0f'));
      expect(
          _hex(ct.sublist(0, 16)), equals('7649abac8119b246cee98e9b12e9197d'));
    });

    test('nist all blocks', () {
      final ct = cbcEncrypt(_unhex(_nistAll), _unhex(_nistKey),
          _unhex('000102030405060708090a0b0c0d0e0f'));
      expect(ct.length, equals(80));
      expect(
          _hex(ct.sublist(0, 16)), equals('7649abac8119b246cee98e9b12e9197d'));
      expect(
          _hex(ct.sublist(16, 32)), equals('5086cb9b507219ee95db113a917678b2'));
      expect(
          _hex(ct.sublist(32, 48)), equals('73bed6b8e3c1743b7116e69e22229516'));
      expect(
          _hex(ct.sublist(48, 64)), equals('3ff1caa1681fac09120eca307586e1a7'));
    });

    test('roundtrip', () {
      final key = _unhex(_nistKey);
      final iv = _unhex('000102030405060708090a0b0c0d0e0f');
      final pt = _unhex(_nistAll);
      expect(cbcDecrypt(cbcEncrypt(pt, key, iv), key, iv), equals(pt));
    });

    test('rejects wrong iv length', () {
      expect(
        () => cbcEncrypt(_unhex(_nistBlock1), _unhex(_nistKey), Uint8List(8)),
        throwsA(equals(CbcInvalidIv(8))),
      );
    });
  });

  group('ctr', () {
    test('roundtrip', () {
      final key = _unhex(_nistKey);
      final nonce = _unhex('f0f1f2f3f4f5f6f7f8f9fafb');
      final pt = _unhex(_nistAll);
      expect(ctrDecrypt(ctrEncrypt(pt, key, nonce), key, nonce), equals(pt));
    });

    test('no padding', () {
      final key = _unhex(_nistKey);
      final nonce = Uint8List(12);
      final pt = Uint8List.fromList([0xde, 0xad]);
      final ct = ctrEncrypt(pt, key, nonce);
      expect(ct.length, equals(2));
      expect(ctrDecrypt(ct, key, nonce), equals(pt));
    });

    test('empty', () {
      final ct = ctrEncrypt(Uint8List(0), _unhex(_nistKey), Uint8List(12));
      expect(ct.length, equals(0));
    });

    test('rejects wrong nonce', () {
      expect(
        () => ctrEncrypt(
            Uint8List.fromList([0]), _unhex(_nistKey), Uint8List(16)),
        throwsA(equals(CtrInvalidNonce(16))),
      );
    });
  });

  group('ghash', () {
    test('multiply fixed nist intermediate', () {
      final h = _unhex('66e94bd4ef8a2c3b884cfa59ca342b2e');
      final x = _unhex('0388dace60b6a392f328c2b971b2fe78');
      final output = Uint8List(16);
      ghashMultiplyFixed(x, h, output);
      expect(_hex(output), equals('5e2ec746917062882c85b0685353deb7'));
    });

    test('fixed nist case 2', () {
      final h = _unhex('66e94bd4ef8a2c3b884cfa59ca342b2e');
      final ciphertext = _unhex('0388dace60b6a392f328c2b971b2fe78');
      final output = Uint8List(16);
      ghashFixed(h, [], ciphertext, output);
      expect(_hex(output), equals('f38cbb1ad69223dcc3457ae5b6b0f885'));
    });

    test('fixed matches reference across block edges', () {
      final h = _unhex('b83b533708bf535d0aa6e52980d53b78');
      final data = Uint8List.fromList(List<int>.generate(256, (i) => i));
      for (final aadLength in [0, 1, 15, 16, 17, 31, 32, 33, 127]) {
        for (final ctLength in [0, 1, 15, 16, 17, 31, 32, 33, 255]) {
          final aad = data.sublist(0, aadLength);
          final ciphertext = data.sublist(0, ctLength);
          final output = Uint8List.fromList(List<int>.filled(16, 0xa5));
          ghashFixed(h, aad, ciphertext, output);
          expect(output, equals(_ghashReference(h, aad, ciphertext)),
              reason: 'aad $aadLength ct $ctLength');
        }
      }
    });

    test('binds key aad and ciphertext', () {
      final h = Uint8List.fromList(List<int>.filled(16, 0x42));
      final baseline = Uint8List(16);
      final changedH = Uint8List(16);
      final changedAad = Uint8List(16);
      final changedCt = Uint8List(16);
      ghashFixed(h, 'header'.codeUnits, 'ciphertext'.codeUnits, baseline);
      final otherH = Uint8List.fromList(h)..[0] ^= 1;
      ghashFixed(otherH, 'header'.codeUnits, 'ciphertext'.codeUnits, changedH);
      ghashFixed(h, 'Header'.codeUnits, 'ciphertext'.codeUnits, changedAad);
      ghashFixed(h, 'header'.codeUnits, 'Ciphertext'.codeUnits, changedCt);
      expect(baseline, isNot(equals(changedH)));
      expect(baseline, isNot(equals(changedAad)));
      expect(baseline, isNot(equals(changedCt)));
    });

    test('rejects unrepresentable bit length', () {
      // White-box on the shared bit-length gate through the
      // public-in-source validator: 2^61 bytes overflows u64 bits.
      expect(
        () => fixedlib.validateFixedAes128GcmLengths(0, 0, 1 << 61),
        throwsA(equals(const GcmLengthOverflow())),
      );
    });
  });

  group('fixed gcm vectors', () {
    test('empty vector', () {
      final key = Uint8List(16);
      final nonce = Uint8List(12);
      final ciphertext = Uint8List(0);
      final tag = Uint8List.fromList(List<int>.filled(16, 0xa5));
      aes128GcmEncryptFixed([], key, nonce, [], ciphertext, tag);
      expect(_hex(tag), equals('58e2fccefa7e3061367f1d57a4e7455a'));

      final plaintext = Uint8List(0);
      aes128GcmDecryptFixed(ciphertext, key, nonce, [], tag, plaintext);
    });

    test('single block vector', () {
      final key = Uint8List(16);
      final nonce = Uint8List(12);
      final plaintext = Uint8List(16);
      final ciphertext = Uint8List.fromList(List<int>.filled(16, 0xa5));
      final tag = Uint8List.fromList(List<int>.filled(16, 0xa5));
      aes128GcmEncryptFixed(plaintext, key, nonce, [], ciphertext, tag);
      expect(_hex(ciphertext), equals('0388dace60b6a392f328c2b971b2fe78'));
      expect(_hex(tag), equals('ab6e47d42cec13bdf53a67b21257bddf'));

      final opened = Uint8List.fromList(List<int>.filled(16, 0xa5));
      aes128GcmDecryptFixed(ciphertext, key, nonce, [], tag, opened);
      expect(opened, equals(plaintext));
    });

    test('aad and partial block vector', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final nonce = _unhex('cafebabefacedbaddecaf888');
      final aad = _unhex('feedfacedeadbeeffeedfacedeadbeefabaddad2');
      final plaintext = _unhex(
          'd9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72'
          '1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39');
      final expectedCt = _unhex(
          '42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e'
          '21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091');
      final ciphertext =
          Uint8List.fromList(List<int>.filled(plaintext.length, 0xa5));
      final tag = Uint8List.fromList(List<int>.filled(16, 0xa5));
      aes128GcmEncryptFixed(plaintext, key, nonce, aad, ciphertext, tag);
      expect(ciphertext, equals(expectedCt));
      expect(_hex(tag), equals('5bc94fbc3221a5db94fae95ae7121a47'));

      final opened =
          Uint8List.fromList(List<int>.filled(plaintext.length, 0xa5));
      aes128GcmDecryptFixed(ciphertext, key, nonce, aad, tag, opened);
      expect(opened, equals(plaintext));
    });
  });

  group('fixed gcm properties', () {
    test('authenticates before releasing plaintext', () {
      final key = Uint8List.fromList(List<int>.filled(16, 0x11));
      final nonce = Uint8List.fromList(List<int>.filled(12, 0x22));
      final plaintext = 'authenticated plaintext crosses block edges'.codeUnits;
      final aad = 'tls-ciphertext-header'.codeUnits;
      final ciphertext = Uint8List(plaintext.length);
      final tag = Uint8List(16);
      aes128GcmEncryptFixed(plaintext, key, nonce, aad, ciphertext, tag);

      for (var mutation = 0; mutation < 3; mutation++) {
        final changedCt = Uint8List.fromList(ciphertext);
        final changedAad = Uint8List.fromList(aad);
        final changedTag = Uint8List.fromList(tag);
        switch (mutation) {
          case 0:
            changedCt[0] ^= 1;
          case 1:
            changedAad[0] ^= 1;
          case 2:
            changedTag[15] ^= 1;
        }
        final output =
            Uint8List.fromList(List<int>.filled(plaintext.length, 0xa5));
        expect(
          () => aes128GcmDecryptFixed(
              changedCt, key, nonce, changedAad, changedTag, output),
          throwsA(equals(const GcmFixedAuthenticationFailed())),
          reason: 'mutation $mutation',
        );
        expect(
            output,
            equals(
                Uint8List.fromList(List<int>.filled(plaintext.length, 0xa5))));
      }
    });

    test('preflight failures leave outputs unchanged', () {
      final key = Uint8List.fromList(List<int>.filled(16, 0x11));
      final nonce = Uint8List.fromList(List<int>.filled(12, 0x22));
      final ciphertext = Uint8List.fromList([0xa5, 0xa5]);
      final tag = Uint8List.fromList(List<int>.filled(16, 0x5a));
      expect(
        () => aes128GcmEncryptFixed(
            'three'.codeUnits, key, nonce, 'aad'.codeUnits, ciphertext, tag),
        throwsA(equals(const GcmOutputLengthMismatch())),
      );
      expect(ciphertext, equals(Uint8List.fromList([0xa5, 0xa5])));
      expect(tag, equals(Uint8List.fromList(List<int>.filled(16, 0x5a))));

      final plaintext = Uint8List.fromList([0xa5, 0xa5]);
      expect(
        () => aes128GcmDecryptFixed('three'.codeUnits, key, nonce,
            'aad'.codeUnits, Uint8List(16), plaintext),
        throwsA(equals(const GcmOutputLengthMismatch())),
      );
      expect(plaintext, equals(Uint8List.fromList([0xa5, 0xa5])));
    });

    test('rejects length and counter exhaustion before work', () {
      final validate = fixedlib.validateFixedAes128GcmLengths;
      expect(
        () => validate(0, 0, 1 << 61),
        throwsA(equals(const GcmLengthOverflow())),
      );
      // (2^32 - 2) blocks × 16 bytes: boundary accepted, +1 rejected.
      const maxPayload = (0xffffffff - 1) * 16;
      expect(
        validate(maxPayload, maxPayload, 0),
        equals((BigInt.zero, BigInt.from(maxPayload) * BigInt.from(8))),
      );
      expect(
        () => validate(maxPayload + 1, maxPayload + 1, 0),
        throwsA(equals(const GcmCounterExhausted())),
      );
    });

    test('roundtrips block edges', () {
      final key = Uint8List.fromList(List<int>.filled(16, 0x42));
      final nonce = Uint8List.fromList(List<int>.filled(12, 0x24));
      final input = Uint8List.fromList(List<int>.generate(256, (i) => i));
      for (final length in [0, 1, 15, 16, 17, 31, 32, 33, 127, 255]) {
        final plaintext = input.sublist(0, length);
        final ciphertext = Uint8List(length);
        final tag = Uint8List(16);
        aes128GcmEncryptFixed(plaintext, key, nonce,
            'associated-data'.codeUnits, ciphertext, tag);
        final opened = Uint8List.fromList(List<int>.filled(length, 0xa5));
        aes128GcmDecryptFixed(
            ciphertext, key, nonce, 'associated-data'.codeUnits, tag, opened);
        expect(opened, equals(plaintext), reason: 'length $length');
      }
    });
  });

  group('compat gcm', () {
    test('nist test case', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      final pt = _unhex(
          'd9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72'
          '1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255');
      const expectedCt =
          '42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e'
          '21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985';
      const expectedTag = '4d5c2af327cd64a62cf35abd2ba6fab4';
      final (ct, tag) = gcmEncrypt(pt, key, iv, []);
      expect(_hex(ct), equals(expectedCt));
      expect(_hex(tag), equals(expectedTag));
    });

    test('roundtrip with aad', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      final pt = _unhex('d9313225f88406e5a55909c5aff5269a');
      final aad = _unhex('feedfacedeadbeeffeedfacedeadbeef');
      final (ct, tag) = gcmEncrypt(pt, key, iv, aad);
      expect(gcmDecrypt(ct, key, iv, aad, tag), equals(pt));
    });

    test('rejects tampered ciphertext', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      final pt = _unhex('d9313225f88406e5a55909c5aff5269a');
      final (ct, tag) = gcmEncrypt(pt, key, iv, []);
      final tampered = Uint8List.fromList(ct)..[0] ^= 0x01;
      expect(
        () => gcmDecrypt(tampered, key, iv, [], tag),
        throwsA(isA<GcmAuthenticationFailed>()),
      );
    });

    test('rejects tampered tag', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      final pt = _unhex('d9313225f88406e5a55909c5aff5269a');
      final (ct, tag) = gcmEncrypt(pt, key, iv, []);
      final tampered = Uint8List.fromList(tag)..[0] ^= 0x01;
      expect(
        () => gcmDecrypt(ct, key, iv, [], tampered),
        throwsA(isA<GcmAuthenticationFailed>()),
      );
    });

    test('empty pt empty aad', () {
      final key = _unhex('00000000000000000000000000000000');
      final iv = _unhex('000000000000000000000000');
      final (ct, tag) = gcmEncrypt([], key, iv, []);
      expect(ct.length, equals(0));
      expect(_hex(tag), equals('58e2fccefa7e3061367f1d57a4e7455a'));
    });

    test('rejects wrong iv', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      expect(
        () => gcmEncrypt([], key, Uint8List(16), []),
        throwsA(equals(GcmInvalidIv(16))),
      );
    });

    test('rejects wrong aad', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      final pt = _unhex('d9313225f88406e5a55909c5aff5269a');
      final aad = _unhex('feedfacedeadbeef');
      final (ct, tag) = gcmEncrypt(pt, key, iv, aad);
      final wrongAad = _unhex('deadbeeffeedface');
      expect(
        () => gcmDecrypt(ct, key, iv, wrongAad, tag),
        throwsA(isA<GcmAuthenticationFailed>()),
      );
    });
  });

  group('lane extras', () {
    test('decrypt-side widths are checked', () {
      final key = _unhex(_nistKey);
      expect(
        () => cbcDecrypt(Uint8List(16), key, Uint8List(15)),
        throwsA(equals(CbcInvalidIv(15))),
      );
      expect(
        () => gcmDecrypt(Uint8List(0), key, Uint8List(11), [], Uint8List(16)),
        throwsA(equals(GcmInvalidIv(11))),
      );
    });

    test('fixed buffer lengths are checked', () {
      final good16 = Uint8List(16);
      final good12 = Uint8List(12);
      expect(() => ghashMultiplyFixed(Uint8List(15), good16, good16),
          throwsArgumentError);
      expect(() => ghashMultiplyFixed(good16, good16, Uint8List(17)),
          throwsArgumentError);
      expect(() => ghashMultiplyFixed(good16, Uint8List(17), good16),
          throwsArgumentError);
      expect(
          () => ghashFixed(Uint8List(15), [], [], good16), throwsArgumentError);
      expect(
          () => ghashFixed(good16, [], [], Uint8List(15)), throwsArgumentError);
      expect(
          () => aes128GcmEncryptFixed(
              [], Uint8List(15), good12, [], Uint8List(0), good16),
          throwsArgumentError);
      expect(
          () => aes128GcmEncryptFixed(
              [], good16, Uint8List(11), [], Uint8List(0), good16),
          throwsArgumentError);
      expect(
          () => aes128GcmEncryptFixed(
              [], good16, good12, [], Uint8List(0), Uint8List(15)),
          throwsArgumentError);
      expect(
          () => aes128GcmDecryptFixed(
              [], Uint8List(15), good12, [], good16, Uint8List(0)),
          throwsArgumentError);
      expect(
          () => aes128GcmDecryptFixed(
              [], good16, Uint8List(11), [], good16, Uint8List(0)),
          throwsArgumentError);
      expect(
          () => aes128GcmDecryptFixed(
              [], good16, good12, [], Uint8List(15), Uint8List(0)),
          throwsArgumentError);
      expect(
          () => aes128GcmDecryptFixed(
              [], good16, good12, [], good16, Uint8List(1)),
          throwsA(equals(const GcmOutputLengthMismatch())));
    });

    test('tag length is checked', () {
      final key = _unhex('feffe9928665731c6d6a8f9467308308');
      final iv = _unhex('cafebabefacedbaddecaf888');
      expect(
        () => gcmDecrypt(Uint8List(0), key, iv, [], Uint8List(15)),
        throwsArgumentError,
      );
    });

    test('error values compare by type and payload', () {
      final errors = [
        const InvalidPaddedLength(),
        const InvalidPadding(),
        const EcbInvalidCiphertext(),
        CbcInvalidIv(8),
        const CbcInvalidCiphertext(),
        CtrInvalidNonce(16),
        GcmInvalidIv(16),
        const GcmAuthenticationFailed(),
        const GhashInputTooLong(),
        const GcmOutputLengthMismatch(),
        const GcmLengthOverflow(),
        const GcmCounterExhausted(),
        const GcmFixedAuthenticationFailed(),
      ];
      for (final error in errors) {
        expect(error == error, isTrue);
        expect(error.hashCode, equals(error.hashCode));
        expect(error.toString(), isNotEmpty);
      }
      for (var i = 0; i < errors.length; i++) {
        for (var j = 0; j < errors.length; j++) {
          if (i != j) {
            expect(errors[i] == errors[j], isFalse, reason: '$i vs $j');
          }
        }
      }
      expect(CbcInvalidIv(8), isNot(equals(CbcInvalidIv(9))));
      expect(CtrInvalidNonce(16), isNot(equals(CtrInvalidNonce(11))));
      expect(GcmInvalidIv(16), isNot(equals(GcmInvalidIv(11))));
    });

    test('exact error messages', () {
      expect(
        const InvalidPaddedLength().toString(),
        equals('Invalid padded data: length must be a positive multiple of 16'),
      );
      expect(
          const InvalidPadding().toString(), equals('Invalid PKCS#7 padding'));
      expect(
        const EcbInvalidCiphertext().toString(),
        equals('ECB ciphertext must be a non-empty multiple of 16 bytes'),
      );
      expect(
          CbcInvalidIv(8).toString(), equals('CBC IV must be 16 bytes, got 8'));
      expect(
        const CbcInvalidCiphertext().toString(),
        equals('CBC ciphertext must be a non-empty multiple of 16 bytes'),
      );
      expect(CtrInvalidNonce(16).toString(),
          equals('CTR nonce must be 12 bytes, got 16'));
      expect(GcmInvalidIv(16).toString(),
          equals('GCM IV must be 12 bytes, got 16'));
      expect(const GcmAuthenticationFailed().toString(),
          equals('GCM authentication failed: tag mismatch'));
      expect(const GhashInputTooLong().toString(),
          equals('GHASH input exceeds the supported length'));
      expect(const GcmOutputLengthMismatch().toString(),
          equals('AES-128-GCM output length must equal input length'));
      expect(const GcmLengthOverflow().toString(),
          equals('AES-128-GCM input length exceeds the GCM encoding'));
      expect(const GcmCounterExhausted().toString(),
          equals('AES-128-GCM request exceeds the 32-bit block counter'));
      expect(const GcmFixedAuthenticationFailed().toString(),
          equals('AES-128-GCM authentication failed'));
    });
  });
}
