/// Unit tests for `coding_adventures_chacha20_poly1305`: all 35
/// reference tests mirrored case-for-case (ChaCha20 vectors and
/// fixed boundaries, Poly1305 vectors and fixed agreement, AEAD
/// vectors and tamper rejection, HChaCha20/XChaCha20 vectors,
/// length preflights), plus lane extras (fixed-size checks, error
/// values).
library;

import 'dart:typed_data';

import 'package:coding_adventures_chacha20_poly1305/chacha20_poly1305.dart';
import 'package:coding_adventures_chacha20_poly1305/src/aead.dart' as aeadlib;
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

Uint8List _filled(int n, int b) => Uint8List.fromList(List<int>.filled(n, b));

Uint8List _range(int n) =>
    Uint8List.fromList(List<int>.generate(n, (i) => i & 0xff));

void main() {
  group('chacha20', () {
    test('rfc8439 section 2.4.2', () {
      final key = _range(32);
      final nonce = Uint8List.fromList([0, 0, 0, 0, 0, 0, 0, 0x4a, 0, 0, 0, 0]);
      final plaintext =
          'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.'
              .codeUnits;
      const expected =
          '6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b'
          'f91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d8'
          '07ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab7793736'
          '5af90bbf74a35be6b40b8eedf2785e42874d';
      final ciphertext = chacha20Encrypt(plaintext, key, nonce, 1);
      expect(_hex(ciphertext), equals(expected));

      final fixed = Uint8List(plaintext.length);
      chacha20XorFixed(plaintext, key, nonce, 1, fixed);
      expect(_hex(fixed), equals(expected));

      expect(chacha20Encrypt(ciphertext, key, nonce, 1), equals(plaintext));
    });

    test('empty', () {
      final result = chacha20Encrypt([], Uint8List(32), Uint8List(12), 0);
      expect(result.isEmpty, isTrue);
    });

    test('single byte', () {
      final key = Uint8List(32)..[0] = 1;
      final ct = chacha20Encrypt([0x42], key, Uint8List(12), 0);
      expect(ct.length, equals(1));
      expect(chacha20Encrypt(ct, key, Uint8List(12), 0), equals([0x42]));
    });

    test('multiblock', () {
      final key = _range(32);
      final nonce = Uint8List(12)..[0] = 0x09;
      final plaintext =
          Uint8List.fromList(List<int>.generate(200, (i) => i % 256));
      final ct = chacha20Encrypt(plaintext, key, nonce, 0);
      expect(ct.length, equals(200));
      expect(chacha20Encrypt(ct, key, nonce, 0), equals(plaintext));
    });

    test('different keys', () {
      final key1 = Uint8List(32)..[0] = 1;
      final key2 = Uint8List(32)..[0] = 2;
      final plaintext = 'Hello, World!'.codeUnits;
      expect(chacha20Encrypt(plaintext, key1, Uint8List(12), 0),
          isNot(equals(chacha20Encrypt(plaintext, key2, Uint8List(12), 0))));
    });

    test('fixed matches legacy across block edges', () {
      final key = _range(32);
      final nonce =
          Uint8List.fromList(List<int>.generate(12, (i) => (i * 7) & 0xff));
      for (final length in [0, 1, 63, 64, 65, 127, 128, 129]) {
        final input =
            Uint8List.fromList(List<int>.generate(length, (i) => i & 0xff));
        final expected = chacha20Encrypt(input, key, nonce, 7);
        final output = Uint8List.fromList(List<int>.filled(length, 0xa5));
        chacha20XorFixed(input, key, nonce, 7, output);
        expect(output, equals(expected), reason: 'length $length');
      }
    });

    test('fixed rejects length mismatch without writing', () {
      final output = Uint8List.fromList(List<int>.filled(7, 0xa5));
      expect(
        () => chacha20XorFixed(
            Uint8List(8), _filled(32, 0x11), _filled(12, 0x22), 0, output),
        throwsA(equals(const ChaCha20OutputLengthMismatch())),
      );
      expect(output, equals(Uint8List.fromList(List<int>.filled(7, 0xa5))));
    });

    test('fixed rejects counter wrap without writing', () {
      final output = Uint8List.fromList(List<int>.filled(65, 0xa5));
      expect(
        () => chacha20XorFixed(Uint8List(65), _filled(32, 0x11),
            _filled(12, 0x22), 0xffffffff, output),
        throwsA(equals(const ChaCha20CounterExhausted())),
      );
      expect(output, equals(Uint8List.fromList(List<int>.filled(65, 0xa5))));
    });

    test('fixed allows last counter for one block', () {
      final input = Uint8List.fromList(List<int>.filled(64, 0x42));
      final output = Uint8List(64);
      chacha20XorFixed(
          input, _filled(32, 0x11), _filled(12, 0x22), 0xffffffff, output);
      expect(output, isNot(equals(input)));
    });

    test('fixed errors disclose no values', () {
      expect(const ChaCha20OutputLengthMismatch().toString(),
          equals('ChaCha20 output length must equal input length'));
      expect(const ChaCha20CounterExhausted().toString(),
          equals('ChaCha20 request exceeds the 32-bit block counter'));
    });

    test('out-of-range counters rejected', () {
      final key = Uint8List(32);
      final nonce = Uint8List(12);
      for (final bad in [-1, 0x100000000]) {
        expect(() => chacha20Encrypt([], key, nonce, bad), throwsArgumentError);
        expect(() => chacha20XorFixed([], key, nonce, bad, Uint8List(0)),
            throwsArgumentError);
        expect(() => xchacha20Encrypt([], key, Uint8List(24), bad),
            throwsArgumentError);
      }
      // Boundary counters still work.
      expect(chacha20Encrypt([], key, nonce, 0), equals(Uint8List(0)));
      expect(chacha20Encrypt([], key, nonce, 0xffffffff), equals(Uint8List(0)));
    });

    test('compat counter wraps mod 2^32 like the reference', () {
      // Deliberate reference parity (not a bug): the compat path
      // wraps where the fixed path throws. Pinned so a future
      // change is a conscious decision, not drift.
      final key = _range(32);
      final nonce = Uint8List(12);
      final wrapped = chacha20Encrypt(Uint8List(65), key, nonce, 0xffffffff);
      final fresh = chacha20Encrypt(Uint8List(1), key, nonce, 0);
      expect(wrapped.sublist(64), equals(fresh));
    });
  });

  group('poly1305', () {
    test('rfc8439 section 2.5.2', () {
      final key = _unhex(
          '85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b');
      final tag =
          poly1305Mac('Cryptographic Forum Research Group'.codeUnits, key);
      expect(_hex(tag), equals('a8061dc1305136c6c22b8baf0c0127a9'));
    });

    test('fixed matches rfc8439 section 2.5.2', () {
      final key = _unhex(
          '85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b');
      final tag = Uint8List.fromList(List<int>.filled(16, 0xa5));
      poly1305MacFixed(
          'Cryptographic Forum Research Group'.codeUnits, key, tag);
      expect(_hex(tag), equals('a8061dc1305136c6c22b8baf0c0127a9'));
    });

    test('fixed matches legacy across block edges', () {
      final key = Uint8List.fromList(
          List<int>.generate(32, (i) => (i * 29 + 7) & 0xff));
      final message = Uint8List.fromList(
          List<int>.generate(257, (i) => (i * 113 + 19) & 0xff));
      for (final length in [
        0,
        1,
        15,
        16,
        17,
        31,
        32,
        33,
        63,
        64,
        65,
        255,
        256,
        257
      ]) {
        final expected = poly1305Mac(message.sublist(0, length), key);
        final actual = Uint8List.fromList(List<int>.filled(16, 0x5a));
        poly1305MacFixed(message.sublist(0, length), key, actual);
        expect(actual, equals(expected),
            reason: 'fixed tag differs at length $length');
      }
    });

    test('fixed matches independent long vector', () {
      final key = Uint8List.fromList(
          List<int>.generate(32, (i) => (i * 29 + 7) & 0xff));
      final message = Uint8List.fromList(
          List<int>.generate(257, (i) => (i * 113 + 19) & 0xff));
      final actual = Uint8List(16);
      poly1305MacFixed(message, key, actual);
      expect(_hex(actual), equals('4860feab5f510294db900e656daee7b3'));
    });

    test('fixed changes with the one-time key', () {
      final message = 'one admitted message'.codeUnits;
      final firstKey = _filled(32, 0x3c);
      final secondKey = Uint8List.fromList(firstKey)..[31] ^= 0x80;
      final firstTag = Uint8List(16);
      final secondTag = Uint8List(16);
      poly1305MacFixed(message, firstKey, firstTag);
      poly1305MacFixed(message, secondKey, secondTag);
      expect(firstTag, isNot(equals(secondTag)));
    });

    test('empty', () {
      final tag = poly1305Mac([], _range(32));
      expect(tag.length, equals(16));
    });

    test('single byte', () {
      final tag = poly1305Mac([0x42], _range(32));
      expect(tag.length, equals(16));
    });

    test('different messages', () {
      final key = _unhex(
          '85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b');
      expect(poly1305Mac('Message A'.codeUnits, key),
          isNot(equals(poly1305Mac('Message B'.codeUnits, key))));
    });

    test('exactly 16 bytes', () {
      final tag = poly1305Mac(_range(16), _range(32));
      expect(tag.length, equals(16));
    });
  });

  group('aead', () {
    test('rfc8439 section 2.8.2', () {
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('070000004041424344454647');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final plaintext =
          'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.'
              .codeUnits;
      const expectedCt =
          'd31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6'
          '3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36'
          '92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc'
          '3ff4def08e4b7a9de576d26586cec64b6116';
      const expectedTag = '1ae10b594f09e26a7e902ecbd0600691';
      final (ciphertext, tag) = aeadEncrypt(plaintext, key, nonce, aad);
      expect(_hex(ciphertext), equals(expectedCt));
      expect(_hex(tag), equals(expectedTag));
    });

    test('round trip', () {
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('070000004041424344454647');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final plaintext =
          'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.'
              .codeUnits;
      final (ciphertext, tag) = aeadEncrypt(plaintext, key, nonce, aad);
      expect(aeadDecrypt(ciphertext, key, nonce, aad, tag), equals(plaintext));
    });

    test('tampered ciphertext', () {
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('070000004041424344454647');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final (ciphertext, tag) =
          aeadEncrypt('Secret message'.codeUnits, key, nonce, aad);
      final tampered = Uint8List.fromList(ciphertext)..[0] ^= 0x01;
      expect(aeadDecrypt(tampered, key, nonce, aad, tag), isNull);
    });

    test('tampered aad', () {
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('070000004041424344454647');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final (ciphertext, tag) =
          aeadEncrypt('Secret message'.codeUnits, key, nonce, aad);
      final tamperedAad = Uint8List.fromList(aad)..[0] ^= 0x01;
      expect(aeadDecrypt(ciphertext, key, nonce, tamperedAad, tag), isNull);
    });

    test('empty plaintext', () {
      final key = _range(32);
      final nonce = Uint8List(12)..[0] = 7;
      final (ct, tag) = aeadEncrypt([], key, nonce, 'header data'.codeUnits);
      expect(ct.isEmpty, isTrue);
      expect(tag.length, equals(16));
      expect(aeadDecrypt(ct, key, nonce, 'header data'.codeUnits, tag),
          equals(Uint8List(0)));
    });

    test('empty aad', () {
      final key = _range(32);
      final plaintext = 'Hello, World!'.codeUnits;
      final (ct, tag) = aeadEncrypt(plaintext, key, Uint8List(12), []);
      expect(aeadDecrypt(ct, key, Uint8List(12), [], tag), equals(plaintext));
    });

    test('large plaintext', () {
      final key = _range(32);
      final nonce = Uint8List(12)..[4] = 0xab;
      final plaintext =
          Uint8List.fromList(List<int>.generate(500, (i) => i % 256));
      final (ct, tag) =
          aeadEncrypt(plaintext, key, nonce, 'extra data'.codeUnits);
      expect(ct.length, equals(500));
      expect(aeadDecrypt(ct, key, nonce, 'extra data'.codeUnits, tag),
          equals(plaintext));
    });

    test('wrong tag', () {
      final (ciphertext, _) =
          aeadEncrypt('test'.codeUnits, Uint8List(32), Uint8List(12), []);
      expect(
          aeadDecrypt(
              ciphertext, Uint8List(32), Uint8List(12), [], Uint8List(16)),
          isNull);
    });

    test('fixed matches rfc8439 section 2.8.2', () {
      // No equivalent exists in the reference suite (it pins only
      // the preflight); this test closes the hole in both lanes —
      // a big-endian length block survived here until the
      // differential campaign caught it.
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('070000004041424344454647');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final plaintext =
          'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.'
              .codeUnits;
      final ciphertext = Uint8List(plaintext.length);
      final tag = Uint8List(16);
      aeadEncryptFixed(plaintext, key, nonce, aad, ciphertext, tag);
      expect(
          _hex(ciphertext),
          equals(
              'd31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6'
              '3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36'
              '92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc'
              '3ff4def08e4b7a9de576d26586cec64b6116'));
      expect(_hex(tag), equals('1ae10b594f09e26a7e902ecbd0600691'));
      final opened = Uint8List(plaintext.length);
      aeadDecryptFixed(ciphertext, key, nonce, aad, tag, opened);
      expect(opened, equals(plaintext));
    });
  });

  group('xchacha20', () {
    test('hchacha20 draft section 2.2.1', () {
      final key = _unhex(
          '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f');
      final nonce = _unhex('000000090000004a0000000031415927');
      expect(
        _hex(hchacha20Subkey(key, nonce)),
        equals(
            '82413b4227b27bfed30e42508a877d73a0f9e4d58a74a853c12ec41326d3ecdc'),
      );
    });

    test('poly1305 aead appendix a3', () {
      final key = _unhex(
          '808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f');
      final nonce = _unhex('404142434445464748494a4b4c4d4e4f5051525354555657');
      final aad = _unhex('50515253c0c1c2c3c4c5c6c7');
      final plaintext =
          'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.'
              .codeUnits;
      const expectedCt =
          'bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb'
          '731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452'
          '2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9'
          '21f9664c97637da9768812f615c68b13b52e';
      const expectedTag = 'c0875924c1c7987947deafd8780acf49';
      final (ct, tag) =
          xchacha20Poly1305AeadEncrypt(plaintext, key, nonce, aad);
      expect(_hex(ct), equals(expectedCt));
      expect(_hex(tag), equals(expectedTag));
      expect(xchacha20Poly1305AeadDecrypt(ct, key, nonce, aad, tag),
          equals(plaintext));
    });

    test('wrong tag rejected', () {
      final (ct, _) = xchacha20Poly1305AeadEncrypt(
          'hello'.codeUnits, Uint8List(32), Uint8List(24), []);
      expect(
          xchacha20Poly1305AeadDecrypt(
              ct, Uint8List(32), Uint8List(24), [], Uint8List(16)),
          isNull);
    });

    test('aad binding', () {
      final key = _filled(32, 7);
      final nonce = _filled(24, 9);
      final (ct, tag) = xchacha20Poly1305AeadEncrypt(
          'secret'.codeUnits, key, nonce, 'ctx-a'.codeUnits);
      expect(
          xchacha20Poly1305AeadDecrypt(ct, key, nonce, 'ctx-b'.codeUnits, tag),
          isNull);
      expect(
          xchacha20Poly1305AeadDecrypt(ct, key, nonce, 'ctx-a'.codeUnits, tag),
          equals('secret'.codeUnits));
    });

    test('tampered ciphertext', () {
      final key = _filled(32, 0x42);
      final nonce = _filled(24, 0x24);
      final (ct, tag) = xchacha20Poly1305AeadEncrypt(
          'do not flip this byte'.codeUnits, key, nonce, []);
      final tampered = Uint8List.fromList(ct)..[5] ^= 0x01;
      expect(
          xchacha20Poly1305AeadDecrypt(tampered, key, nonce, [], tag), isNull);
    });

    test('long message', () {
      final key = _filled(32, 0x11);
      final nonce = _filled(24, 0x22);
      final plaintext =
          Uint8List.fromList(List<int>.generate(4096, (i) => i % 251));
      final (ct, tag) =
          xchacha20Poly1305AeadEncrypt(plaintext, key, nonce, 'ctx'.codeUnits);
      expect(ct.length, equals(plaintext.length));
      expect(xchacha20Poly1305AeadDecrypt(ct, key, nonce, 'ctx'.codeUnits, tag),
          equals(plaintext));
    });

    test('stream round trip', () {
      final key = _filled(32, 0x5a);
      final nonce = _filled(24, 0xa5);
      final plaintext = 'raw stream cipher test, no AEAD'.codeUnits;
      final ct = xchacha20Encrypt(plaintext, key, nonce, 1);
      expect(ct, isNot(equals(plaintext)));
      expect(xchacha20Encrypt(ct, key, nonce, 1), equals(plaintext));
    });
  });

  group('length preflight', () {
    test('enforces counter space without allocating', () {
      final validate = aeadlib.validateAeadRequestLengths;
      final map = aeadlib.mapFixedChacha20Error;
      final maximumLength = 0xffffffff * 64;
      expect(
        validate(maximumLength, maximumLength, 1 << 62),
        equals((BigInt.from(1 << 62), BigInt.from(maximumLength))),
      );
      expect(
        () => validate(maximumLength + 1, maximumLength + 1, 0),
        throwsA(equals(const AeadCounterExhausted())),
      );
      expect(
        () => validate(maximumLength + 1, maximumLength, 0),
        throwsA(equals(const AeadOutputLengthMismatch())),
      );
      expect(
        map(const ChaCha20OutputLengthMismatch()),
        equals(const AeadOutputLengthMismatch()),
      );
      expect(
        map(const ChaCha20CounterExhausted()),
        equals(const AeadCounterExhausted()),
      );
    });
  });

  group('lane extras', () {
    test('fixed sizes are checked', () {
      final key32 = Uint8List(32);
      final nonce12 = Uint8List(12);
      final nonce24 = Uint8List(24);
      final tag16 = Uint8List(16);
      expect(() => chacha20Encrypt([], Uint8List(31), nonce12, 0),
          throwsArgumentError);
      expect(() => chacha20Encrypt([], key32, Uint8List(11), 0),
          throwsArgumentError);
      expect(() => chacha20XorFixed([], key32, nonce12, 0, Uint8List(1)),
          throwsA(equals(const ChaCha20OutputLengthMismatch())));
      expect(() => hchacha20Subkey(Uint8List(31), Uint8List(16)),
          throwsArgumentError);
      expect(() => hchacha20Subkey(key32, Uint8List(15)), throwsArgumentError);
      expect(() => poly1305Mac([], Uint8List(31)), throwsArgumentError);
      expect(() => poly1305MacFixed([], Uint8List(31), tag16),
          throwsArgumentError);
      expect(() => poly1305MacFixed([], key32, Uint8List(15)),
          throwsArgumentError);
      expect(() => aeadEncrypt([], Uint8List(31), nonce12, []),
          throwsArgumentError);
      expect(
          () => aeadEncrypt([], key32, Uint8List(11), []), throwsArgumentError);
      expect(() => aeadDecrypt([], Uint8List(31), nonce12, [], tag16),
          throwsArgumentError);
      expect(() => aeadDecrypt([], key32, nonce12, [], Uint8List(15)),
          throwsArgumentError);
      expect(
          () => aeadEncryptFixed(
              [], Uint8List(31), nonce12, [], Uint8List(0), tag16),
          throwsArgumentError);
      expect(
          () => aeadEncryptFixed(
              [], key32, nonce12, [], Uint8List(0), Uint8List(15)),
          throwsArgumentError);
      expect(
          () => aeadEncryptFixed([], key32, nonce12, [], Uint8List(1), tag16),
          throwsA(equals(const AeadOutputLengthMismatch())));
      expect(
          () => aeadDecryptFixed(
              [], Uint8List(31), nonce12, [], tag16, Uint8List(0)),
          throwsArgumentError);
      expect(
          () => aeadDecryptFixed(
              [], key32, nonce12, [], Uint8List(15), Uint8List(0)),
          throwsArgumentError);
      expect(() => xchacha20Encrypt([], Uint8List(31), nonce24, 0),
          throwsArgumentError);
      expect(() => xchacha20Encrypt([], key32, Uint8List(23), 0),
          throwsArgumentError);
      expect(() => xchacha20Poly1305AeadEncrypt([], Uint8List(31), nonce24, []),
          throwsArgumentError);
      expect(() => xchacha20Poly1305AeadEncrypt([], key32, Uint8List(23), []),
          throwsArgumentError);
      expect(
          () => xchacha20Poly1305AeadDecrypt(
              [], Uint8List(31), nonce24, [], tag16),
          throwsArgumentError);
      expect(
          () =>
              xchacha20Poly1305AeadDecrypt([], key32, Uint8List(23), [], tag16),
          throwsArgumentError);
      expect(
          () => xchacha20Poly1305AeadDecrypt(
              [], key32, nonce24, [], Uint8List(15)),
          throwsArgumentError);
    });

    test('error values compare by type', () {
      final errors = [
        const ChaCha20OutputLengthMismatch(),
        const ChaCha20CounterExhausted(),
        const AeadOutputLengthMismatch(),
        const AeadLengthOverflow(),
        const AeadCounterExhausted(),
        const AeadAuthenticationFailed(),
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
      expect(const ChaCha20OutputLengthMismatch().toString(),
          equals('ChaCha20 output length must equal input length'));
      expect(const ChaCha20CounterExhausted().toString(),
          equals('ChaCha20 request exceeds the 32-bit block counter'));
      expect(const AeadOutputLengthMismatch().toString(),
          equals('AEAD output length must equal input length'));
      expect(const AeadLengthOverflow().toString(),
          equals('AEAD input length exceeds the RFC 8439 encoding'));
      expect(const AeadCounterExhausted().toString(),
          equals('AEAD request exceeds the ChaCha20 block counter'));
      expect(const AeadAuthenticationFailed().toString(),
          equals('AEAD authentication failed'));
    });
  });
}
