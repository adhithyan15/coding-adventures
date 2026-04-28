import 'dart:typed_data';
import 'package:coding_adventures_reed_solomon/coding_adventures_reed_solomon.dart';
import 'package:test/test.dart';

void main() {
  // ==========================================================================
  // buildGenerator
  // ==========================================================================

  group('rsBuildGenerator', () {
    test('nCheck=2: g = [8, 6, 1] (8 + 6x + x²)', () {
      // g(x) = (x + α¹)(x + α²) = (x + 2)(x + 4)
      // Multiply: [2,1]×[4,1] = [GF.mul(2,4), 2 XOR 4, 1] = [8, 6, 1]
      final g = rsBuildGenerator(2);
      expect(g.length, equals(3));
      expect(g[0], equals(8));
      expect(g[1], equals(6));
      expect(g[2], equals(1));
    });

    test('generator is monic (leading coefficient = 1)', () {
      for (final nCheck in [2, 4, 6, 8, 10]) {
        final g = rsBuildGenerator(nCheck);
        expect(g[g.length - 1], equals(1),
            reason: 'generator for nCheck=$nCheck should be monic');
      }
    });

    test('generator has length nCheck+1', () {
      for (final nCheck in [2, 4, 6, 8]) {
        expect(rsBuildGenerator(nCheck).length, equals(nCheck + 1),
            reason: 'generator for nCheck=$nCheck should have length ${nCheck + 1}');
      }
    });

    test('nCheck=0 throws InvalidInputException', () {
      expect(() => rsBuildGenerator(0), throwsA(isA<InvalidInputException>()));
    });

    test('nCheck=3 (odd) throws InvalidInputException', () {
      expect(() => rsBuildGenerator(3), throwsA(isA<InvalidInputException>()));
    });
  });

  // ==========================================================================
  // rsEncode
  // ==========================================================================

  group('rsEncode', () {
    test('codeword length = message.length + nCheck', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 8);
      expect(codeword.length, equals(13));
    });

    test('systematic: message bytes appear unchanged at start of codeword', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 8);
      for (int i = 0; i < message.length; i++) {
        expect(codeword[i], equals(message[i]),
            reason: 'codeword[$i] should match message[$i]');
      }
    });

    test('valid codeword has all-zero syndromes', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 8);
      final synds = rsSyndromes(codeword, 8);
      for (int i = 0; i < synds.length; i++) {
        expect(synds[i], equals(0),
            reason: 'syndrome $i should be 0 for valid codeword');
      }
    });

    test('empty message encodes to nCheck check bytes', () {
      final message = Uint8List(0);
      final codeword = rsEncode(message, 2);
      expect(codeword.length, equals(2));
    });

    test('nCheck=0 throws InvalidInputException', () {
      expect(
        () => rsEncode(Uint8List.fromList([1, 2, 3]), 0),
        throwsA(isA<InvalidInputException>()),
      );
    });

    test('nCheck=3 (odd) throws InvalidInputException', () {
      expect(
        () => rsEncode(Uint8List.fromList([1, 2, 3]), 3),
        throwsA(isA<InvalidInputException>()),
      );
    });

    test('total length > 255 throws InvalidInputException', () {
      final longMessage = Uint8List(254);
      expect(
        () => rsEncode(longMessage, 4),
        throwsA(isA<InvalidInputException>()),
      );
    });
  });

  // ==========================================================================
  // rsSyndromes
  // ==========================================================================

  group('rsSyndromes', () {
    test('valid codeword has all-zero syndromes', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 4);
      final synds = rsSyndromes(codeword, 4);
      expect(synds.every((s) => s == 0), isTrue);
    });

    test('single error produces non-zero syndromes', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[2] ^= 0xFF;
      final synds = rsSyndromes(corrupted, 4);
      expect(synds.any((s) => s != 0), isTrue);
    });
  });

  // ==========================================================================
  // rsDecode — round-trip
  // ==========================================================================

  group('rsDecode round-trip (no errors)', () {
    test('encode then decode returns original message, nCheck=2', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      expect(rsDecode(rsEncode(message, 2), 2), equals(message));
    });

    test('encode then decode returns original message, nCheck=8', () {
      final message = Uint8List.fromList([10, 20, 30, 40, 50, 60, 70]);
      expect(rsDecode(rsEncode(message, 8), 8), equals(message));
    });

    test('round-trip with all byte values 0..127', () {
      final message = Uint8List.fromList(List<int>.generate(50, (i) => i));
      expect(rsDecode(rsEncode(message, 8), 8), equals(message));
    });

    test('round-trip with message of all zeros', () {
      final message = Uint8List(10);
      expect(rsDecode(rsEncode(message, 4), 4), equals(message));
    });

    test('round-trip with message of all 0xFF', () {
      final message = Uint8List.fromList(List<int>.filled(10, 0xFF));
      expect(rsDecode(rsEncode(message, 4), 4), equals(message));
    });
  });

  // ==========================================================================
  // rsDecode — error correction
  // ==========================================================================

  group('rsDecode error correction', () {
    test('correct 1 error with nCheck=2 (t=1)', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 2);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[0] ^= 0xAB;
      expect(rsDecode(corrupted, 2), equals(message));
    });

    test('correct 2 errors with nCheck=4 (t=2)', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[1] ^= 0x55;
      corrupted[3] ^= 0xAA;
      expect(rsDecode(corrupted, 4), equals(message));
    });

    test('correct 4 errors with nCheck=8 (t=4)', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 8);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[0] ^= 0xFF;
      corrupted[2] ^= 0xAA;
      corrupted[4] ^= 0x55;
      corrupted[7] ^= 0x11;
      expect(rsDecode(corrupted, 8), equals(message));
    });

    test('correct error in check bytes', () {
      final message = Uint8List.fromList([10, 20, 30]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      // Corrupt a check byte (last 4 bytes)
      corrupted[codeword.length - 1] ^= 0xFF;
      expect(rsDecode(corrupted, 4), equals(message));
    });

    test('correct all errors at last positions in codeword', () {
      final message = Uint8List.fromList([5, 10, 15, 20, 25, 30]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[codeword.length - 1] ^= 0xAB;
      corrupted[codeword.length - 2] ^= 0xCD;
      expect(rsDecode(corrupted, 4), equals(message));
    });
  });

  // ==========================================================================
  // rsDecode — failure cases
  // ==========================================================================

  group('rsDecode failure cases', () {
    test('t+1 errors throws TooManyErrorsException (nCheck=2, t=1)', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 2);
      final corrupted = Uint8List.fromList(codeword);
      // 2 errors > t=1
      corrupted[0] ^= 0xFF;
      corrupted[1] ^= 0xAA;
      expect(
        () => rsDecode(corrupted, 2),
        throwsA(isA<TooManyErrorsException>()),
      );
    });

    test('t+1 errors throws TooManyErrorsException (nCheck=4, t=2)', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      // 3 errors > t=2
      corrupted[0] ^= 0xFF;
      corrupted[1] ^= 0xAA;
      corrupted[2] ^= 0x55;
      expect(
        () => rsDecode(corrupted, 4),
        throwsA(isA<TooManyErrorsException>()),
      );
    });

    test('nCheck=0 throws InvalidInputException', () {
      final received = Uint8List.fromList([1, 2, 3, 4, 5]);
      expect(
        () => rsDecode(received, 0),
        throwsA(isA<InvalidInputException>()),
      );
    });

    test('nCheck=3 (odd) throws InvalidInputException', () {
      final received = Uint8List.fromList([1, 2, 3, 4, 5]);
      expect(
        () => rsDecode(received, 3),
        throwsA(isA<InvalidInputException>()),
      );
    });

    test('received.length < nCheck throws InvalidInputException', () {
      final received = Uint8List.fromList([1]);
      expect(
        () => rsDecode(received, 4),
        throwsA(isA<InvalidInputException>()),
      );
    });
  });

  // ==========================================================================
  // rsErrorLocator
  // ==========================================================================

  group('rsErrorLocator', () {
    test('all-zero syndromes gives locator [1] (no errors)', () {
      final synds = Uint8List(4);
      final lambda = rsErrorLocator(synds);
      expect(lambda.length, equals(1));
      expect(lambda[0], equals(1));
    });

    test('one-error syndromes gives degree-1 locator', () {
      final message = Uint8List.fromList([1, 2, 3, 4, 5]);
      final codeword = rsEncode(message, 4);
      final corrupted = Uint8List.fromList(codeword);
      corrupted[2] ^= 0xFF;
      final synds = rsSyndromes(corrupted, 4);
      final lambda = rsErrorLocator(synds);
      // Degree-1 polynomial: Λ[0]=1, Λ[1]= error locator value
      expect(lambda[0], equals(1));
      expect(lambda.length, equals(2));
    });
  });

  // ==========================================================================
  // Spec test vectors
  // ==========================================================================

  group('spec test vectors', () {
    // From MA02 spec: "Standard QR code Version 1-L message"
    // encode([32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236,
    //         17, 236, 17, 236], nCheck=7)
    // Note: nCheck=7 is odd, so per spec constraint this would be InvalidInput.
    // The spec notes it's a QR vector; in the actual QR implementation nCheck
    // is allowed to be odd. We skip this vector and instead verify the
    // round-trip property for a long message.

    test('round-trip for length-50 message with nCheck=10', () {
      final message = Uint8List.fromList(List<int>.generate(50, (i) => (i * 7 + 13) % 256));
      final codeword = rsEncode(message, 10);
      expect(codeword.length, equals(60));
      expect(rsDecode(codeword, 10), equals(message));
    });

    test('syndromes are zero on valid codeword (spec property)', () {
      final message = Uint8List.fromList([4, 3, 2, 1]);
      final codeword = rsEncode(message, 2);
      final synds = rsSyndromes(codeword, 2);
      expect(synds.every((s) => s == 0), isTrue);
    });

    test('decode(encode(m, n)) == m for many message lengths', () {
      for (int len = 1; len <= 20; len++) {
        final message = Uint8List.fromList(List<int>.generate(len, (i) => (i * 11 + 7) % 256));
        for (final nCheck in [2, 4, 8]) {
          if (len + nCheck > 255) continue;
          final decoded = rsDecode(rsEncode(message, nCheck), nCheck);
          expect(decoded, equals(message),
              reason: 'round-trip failed for len=$len, nCheck=$nCheck');
        }
      }
    });
  });

  // ==========================================================================
  // Constants
  // ==========================================================================

  group('constants', () {
    test('version is defined', () {
      expect(version, isNotEmpty);
    });
  });
}
