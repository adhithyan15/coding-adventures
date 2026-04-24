import 'package:coding_adventures_gf256/coding_adventures_gf256.dart';
import 'package:test/test.dart';

void main() {
  // ==========================================================================
  // Table construction
  // ==========================================================================

  group('log/alog tables', () {
    test('alog[0] = 1 (2^0 = 1)', () {
      expect(alog[0], equals(1));
    });

    test('alog[1] = 2 (2^1 = 2)', () {
      expect(alog[1], equals(2));
    });

    test('alog[7] = 128 (2^7 = 0x80)', () {
      expect(alog[7], equals(128));
    });

    test('alog[8] = 29 (first reduction: 256 XOR 0x11D = 0x1D = 29)', () {
      // At step 8: 128 * 2 = 256 ≥ 256, so 256 XOR 0x11D = 256 XOR 285 = 29.
      expect(alog[8], equals(29));
    });

    test('alog[255] = 1 (multiplicative group wraps: g^255 = g^0 = 1)', () {
      // The multiplicative group of GF(256)\{0} has order 255, so g^255 = 1.
      expect(alog[255], equals(1));
    });

    test('log[1] = 0 (2^0 = 1)', () {
      expect(log[1], equals(0));
    });

    test('log[2] = 1 (2^1 = 2)', () {
      expect(log[2], equals(1));
    });

    test('alog and log are inverses for all non-zero elements', () {
      // For every x in 1..255: alog[log[x]] = x
      for (int x = 1; x <= 255; x++) {
        expect(alog[log[x]], equals(x), reason: 'alog[log[$x]] should be $x');
      }
    });

    test('alog covers all 255 non-zero elements exactly once', () {
      // The generator 2 is primitive: its powers are all 255 non-zero elements.
      final set = <int>{};
      for (int i = 0; i < 255; i++) {
        expect(alog[i], greaterThan(0), reason: 'alog[$i] should be non-zero');
        expect(alog[i], lessThan(256), reason: 'alog[$i] should be < 256');
        set.add(alog[i]);
      }
      expect(set.length, equals(255), reason: 'all 255 non-zero elements should appear');
    });
  });

  // ==========================================================================
  // Add / Subtract
  // ==========================================================================

  group('gfAdd and gfSubtract', () {
    test('add(a, b) = a XOR b', () {
      expect(gfAdd(0x53, 0xCA), equals(0x53 ^ 0xCA));
      expect(gfAdd(0xFF, 0xFF), equals(0));
      expect(gfAdd(0x00, 0x99), equals(0x99));
      expect(gfAdd(0x12, 0x34), equals(0x12 ^ 0x34));
    });

    test('add(x, x) = 0 for all x (characteristic 2: every element is own inverse)', () {
      for (int x = 0; x < 256; x++) {
        expect(gfAdd(x, x), equals(0), reason: 'add($x, $x) should be 0');
      }
    });

    test('add(x, 0) = x (additive identity)', () {
      for (int x = 0; x < 256; x++) {
        expect(gfAdd(x, 0), equals(x));
        expect(gfAdd(0, x), equals(x));
      }
    });

    test('subtract equals add', () {
      // In characteristic 2, -a = a, so a - b = a + b = a XOR b.
      for (int a = 0; a < 256; a += 17) {
        for (int b = 0; b < 256; b += 13) {
          expect(gfSubtract(a, b), equals(gfAdd(a, b)));
        }
      }
    });

    test('add is commutative', () {
      expect(gfAdd(0x12, 0x34), equals(gfAdd(0x34, 0x12)));
      expect(gfAdd(0xAB, 0xCD), equals(gfAdd(0xCD, 0xAB)));
    });

    test('add is associative', () {
      int a = 0x12, b = 0x34, c = 0x56;
      expect(gfAdd(gfAdd(a, b), c), equals(gfAdd(a, gfAdd(b, c))));
    });
  });

  // ==========================================================================
  // Multiply
  // ==========================================================================

  group('gfMultiply', () {
    test('multiply(0, x) = 0 for all x', () {
      for (int x = 0; x < 256; x++) {
        expect(gfMultiply(0, x), equals(0), reason: 'multiply(0, $x) should be 0');
        expect(gfMultiply(x, 0), equals(0), reason: 'multiply($x, 0) should be 0');
      }
    });

    test('multiply(x, 1) = x for all x (multiplicative identity)', () {
      for (int x = 0; x < 256; x++) {
        expect(gfMultiply(x, 1), equals(x), reason: 'multiply($x, 1) should be $x');
        expect(gfMultiply(1, x), equals(x), reason: 'multiply(1, $x) should be $x');
      }
    });

    test('multiply(2, 128) = 29 (demonstrates modular reduction)', () {
      // 2 * 128 = 256. Since 256 >= 256, XOR with 0x11D = 285: 256 XOR 285 = 29.
      // This is alog[1 + 7] = alog[8] = 29.
      expect(gfMultiply(2, 128), equals(29));
    });

    test('multiply is commutative', () {
      for (int a = 0; a < 256; a += 23) {
        for (int b = 0; b < 256; b += 19) {
          expect(gfMultiply(a, b), equals(gfMultiply(b, a)),
              reason: 'multiply($a, $b) should equal multiply($b, $a)');
        }
      }
    });

    test('multiply is associative', () {
      int a = 0x12, b = 0x34, c = 0x56;
      expect(gfMultiply(gfMultiply(a, b), c),
          equals(gfMultiply(a, gfMultiply(b, c))));
    });

    test('multiply distributes over add', () {
      // a * (b + c) = a*b + a*c  (where + = XOR)
      int a = 0xAB, b = 0xCD, c = 0xEF;
      expect(gfMultiply(a, gfAdd(b, c)),
          equals(gfAdd(gfMultiply(a, b), gfMultiply(a, c))));
    });

    test('GF(256) spot check: 0x53 × 0x8C = 0x01', () {
      // 0x53 is the multiplicative inverse of 0x8C under the 0x11D polynomial.
      // (Note: under AES polynomial 0x11B, the pair is 0x53 × 0xCA = 0x01.)
      expect(gfMultiply(0x53, 0x8C), equals(1));
    });
  });

  // ==========================================================================
  // Divide
  // ==========================================================================

  group('gfDivide', () {
    test('divide(0, x) = 0 for all non-zero x', () {
      for (int x = 1; x < 256; x++) {
        expect(gfDivide(0, x), equals(0));
      }
    });

    test('divide(x, 1) = x for all x (dividing by identity)', () {
      for (int x = 1; x < 256; x++) {
        expect(gfDivide(x, 1), equals(x));
      }
    });

    test('divide(x, x) = 1 for all non-zero x', () {
      for (int x = 1; x < 256; x++) {
        expect(gfDivide(x, x), equals(1), reason: 'divide($x, $x) should be 1');
      }
    });

    test('divide(multiply(a, b), b) = a for non-zero b', () {
      for (int a = 1; a < 256; a += 31) {
        for (int b = 1; b < 256; b += 29) {
          expect(gfDivide(gfMultiply(a, b), b), equals(a),
              reason: 'divide(multiply($a, $b), $b) should be $a');
        }
      }
    });

    test('divide(a, 0) throws ArgumentError', () {
      expect(() => gfDivide(1, 0), throwsArgumentError);
      expect(() => gfDivide(0, 0), throwsArgumentError);
    });
  });

  // ==========================================================================
  // Power
  // ==========================================================================

  group('gfPower', () {
    test('power(x, 0) = 1 for all x (including 0)', () {
      for (int x = 0; x < 256; x++) {
        expect(gfPower(x, 0), equals(1));
      }
    });

    test('power(0, n) = 0 for n > 0', () {
      for (int n = 1; n <= 10; n++) {
        expect(gfPower(0, n), equals(0));
      }
    });

    test('power(x, 1) = x for all x', () {
      for (int x = 0; x < 256; x++) {
        expect(gfPower(x, 1), equals(x));
      }
    });

    test('power(2, i) matches alog[i] for i in 0..254', () {
      // The generator g = 2. power(2, i) should equal alog[i].
      for (int i = 0; i <= 254; i++) {
        expect(gfPower(2, i), equals(alog[i]),
            reason: 'power(2, $i) should equal alog[$i]');
      }
    });

    test('power(g, 255) = 1 (multiplicative group has order 255)', () {
      // g^255 = 1 for the generator g = 2.
      expect(gfPower(2, 255), equals(1));
    });

    test('power with negative exp throws ArgumentError', () {
      expect(() => gfPower(2, -1), throwsArgumentError);
    });
  });

  // ==========================================================================
  // Inverse
  // ==========================================================================

  group('gfInverse', () {
    test('x × inverse(x) = 1 for all non-zero x', () {
      for (int x = 1; x <= 255; x++) {
        expect(gfMultiply(x, gfInverse(x)), equals(1),
            reason: '$x × inverse($x) should be 1');
      }
    });

    test('inverse(1) = 1', () {
      expect(gfInverse(1), equals(1));
    });

    test('inverse(inverse(x)) = x', () {
      for (int x = 1; x <= 255; x++) {
        expect(gfInverse(gfInverse(x)), equals(x));
      }
    });

    test('inverse(0) throws ArgumentError', () {
      expect(() => gfInverse(0), throwsArgumentError);
    });
  });

  // ==========================================================================
  // Constants
  // ==========================================================================

  group('constants', () {
    test('gfZero() = 0', () {
      expect(gfZero(), equals(0));
    });

    test('gfOne() = 1', () {
      expect(gfOne(), equals(1));
    });

    test('primitivePolynomial = 0x11D', () {
      expect(primitivePolynomial, equals(0x11D));
    });

    test('version is defined', () {
      expect(version, isNotEmpty);
    });
  });
}
