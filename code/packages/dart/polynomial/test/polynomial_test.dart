import 'package:coding_adventures_polynomial/coding_adventures_polynomial.dart';
import 'package:test/test.dart';

/// Helper to check deep list equality.
void expectPoly(Polynomial actual, List<num> expected) {
  expect(actual.length, equals(expected.length),
      reason: 'lengths differ: got $actual, expected $expected');
  for (int i = 0; i < expected.length; i++) {
    expect(actual[i], closeTo(expected[i], 1e-10),
        reason: 'index $i: got $actual, expected $expected');
  }
}

void main() {
  // ==========================================================================
  // normalize
  // ==========================================================================

  group('normalize', () {
    test('strips trailing zeros', () {
      expectPoly(normalize([1, 0, 0]), [1]);
      expectPoly(normalize([0]), []);
      expectPoly(normalize([0, 0, 0]), []);
    });

    test('returns empty list for zero polynomial', () {
      expect(normalize([]), isEmpty);
      expect(normalize([0]), isEmpty);
    });

    test('leaves already-normalized polynomial unchanged', () {
      expectPoly(normalize([1, 2, 3]), [1, 2, 3]);
      expectPoly(normalize([7]), [7]);
    });

    test('strips only trailing zeros, not leading ones', () {
      // [0, 1] means 0 + 1·x = x — should remain [0, 1]
      expectPoly(normalize([0, 1]), [0, 1]);
      expectPoly(normalize([0, 0, 3]), [0, 0, 3]);
    });
  });

  // ==========================================================================
  // degree
  // ==========================================================================

  group('polynomialDegree', () {
    test('degree of zero polynomial is -1', () {
      expect(polynomialDegree([]), equals(-1));
      expect(polynomialDegree([0]), equals(-1));
      expect(polynomialDegree([0, 0, 0]), equals(-1));
    });

    test('degree of constant polynomial is 0', () {
      expect(polynomialDegree([7]), equals(0));
      expect(polynomialDegree([1]), equals(0));
    });

    test('degree of linear polynomial is 1', () {
      expect(polynomialDegree([0, 5]), equals(1));
      expect(polynomialDegree([3, 2]), equals(1));
    });

    test('degree([3, 0, 2]) = 2 (highest non-zero at index 2)', () {
      expect(polynomialDegree([3, 0, 2]), equals(2));
    });

    test('degree ignores trailing zeros', () {
      expect(polynomialDegree([3, 0, 2, 0, 0]), equals(2));
    });
  });

  // ==========================================================================
  // zero and one
  // ==========================================================================

  group('polynomialZero and polynomialOne', () {
    test('zero() returns empty list', () {
      expect(polynomialZero(), isEmpty);
    });

    test('one() returns [1]', () {
      expectPoly(polynomialOne(), [1]);
    });

    test('add(zero(), p) = p', () {
      final p = [3, 1, 2];
      expectPoly(polynomialAdd(polynomialZero(), p), p);
      expectPoly(polynomialAdd(p, polynomialZero()), p);
    });

    test('multiply(one(), p) = p', () {
      final p = [3, 1, 2];
      expectPoly(polynomialMultiply(polynomialOne(), p), p);
      expectPoly(polynomialMultiply(p, polynomialOne()), p);
    });
  });

  // ==========================================================================
  // add
  // ==========================================================================

  group('polynomialAdd', () {
    test('basic add: [1,2,3] + [4,5] = [5,7,3]', () {
      expectPoly(polynomialAdd([1, 2, 3], [4, 5]), [5, 7, 3]);
    });

    test('add with zero polynomial', () {
      expectPoly(polynomialAdd([1, 2], []), [1, 2]);
      expectPoly(polynomialAdd([], [3, 4, 5]), [3, 4, 5]);
    });

    test('add that produces trailing zeros normalizes', () {
      // [1, 2, 3] + [-1, -2, -3] = [0, 0, 0] → []
      expectPoly(polynomialAdd([1, 2, 3], [-1, -2, -3]), []);
    });

    test('add is commutative', () {
      final a = [1, 2, 3];
      final b = [4, 5];
      expectPoly(polynomialAdd(a, b), polynomialAdd(b, a));
    });

    test('add is associative', () {
      final a = [1, 2];
      final b = [3, 4, 5];
      final c = [6];
      expectPoly(
        polynomialAdd(polynomialAdd(a, b), c),
        polynomialAdd(a, polynomialAdd(b, c)),
      );
    });
  });

  // ==========================================================================
  // subtract
  // ==========================================================================

  group('polynomialSubtract', () {
    test('basic subtract: [5,7,3] - [1,2,3] = [4,5]', () {
      // [5,7,3] - [1,2,3] = [4,5,0] → normalize → [4,5]
      expectPoly(polynomialSubtract([5, 7, 3], [1, 2, 3]), [4, 5]);
    });

    test('subtract polynomial from itself gives zero', () {
      final p = [3, 1, 4, 1];
      expectPoly(polynomialSubtract(p, p), []);
    });

    test('subtract zero polynomial', () {
      expectPoly(polynomialSubtract([1, 2, 3], []), [1, 2, 3]);
    });

    test('subtract from zero gives negation', () {
      expectPoly(polynomialSubtract([], [1, 2, 3]), [-1, -2, -3]);
    });
  });

  // ==========================================================================
  // multiply
  // ==========================================================================

  group('polynomialMultiply', () {
    test('basic multiply: [1,2] × [3,4] = [3,10,8]', () {
      // (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
      expectPoly(polynomialMultiply([1, 2], [3, 4]), [3, 10, 8]);
    });

    test('multiply by zero gives zero', () {
      expectPoly(polynomialMultiply([1, 2, 3], []), []);
      expectPoly(polynomialMultiply([], [4, 5]), []);
    });

    test('multiply by one is identity', () {
      expectPoly(polynomialMultiply([1, 2, 3], [1]), [1, 2, 3]);
      expectPoly(polynomialMultiply([1], [1, 2, 3]), [1, 2, 3]);
    });

    test('multiply constants: [3] × [4] = [12]', () {
      expectPoly(polynomialMultiply([3], [4]), [12]);
    });

    test('multiply is commutative', () {
      final a = [1, 2, 3];
      final b = [4, 5];
      expectPoly(polynomialMultiply(a, b), polynomialMultiply(b, a));
    });

    test('multiply is associative', () {
      final a = [1, 2];
      final b = [3, 4];
      final c = [5, 6];
      expectPoly(
        polynomialMultiply(polynomialMultiply(a, b), c),
        polynomialMultiply(a, polynomialMultiply(b, c)),
      );
    });

    test('multiply distributes over add', () {
      // a*(b+c) = a*b + a*c
      final a = [1, 2];
      final b = [3, 4];
      final c = [5, 6];
      expectPoly(
        polynomialMultiply(a, polynomialAdd(b, c)),
        polynomialAdd(polynomialMultiply(a, b), polynomialMultiply(a, c)),
      );
    });

    test('degree(a*b) = degree(a) + degree(b)', () {
      final a = [1, 0, 1]; // 1 + x²  (degree 2)
      final b = [1, 1];    // 1 + x   (degree 1)
      final product = polynomialMultiply(a, b);
      expect(polynomialDegree(product),
          equals(polynomialDegree(a) + polynomialDegree(b)));
    });
  });

  // ==========================================================================
  // divmod
  // ==========================================================================

  group('polynomialDivmod', () {
    test('basic division: [5,1,3,2] ÷ [2,1] = q=[3,-1,2], r=[-1]', () {
      // 5 + x + 3x² + 2x³  divided by  2 + x
      // From the spec: quotient = 3 - x + 2x², remainder = -1
      final (q, r) = polynomialDivmod([5, 1, 3, 2], [2, 1]);
      expectPoly(q, [3, -1, 2]);
      expectPoly(r, [-1]);
    });

    test('verify: a = b*q + r', () {
      final a = [5, 1, 3, 2];
      final b = [2, 1];
      final (q, r) = polynomialDivmod(a, b);
      final bq = polynomialAdd(polynomialMultiply(b, q), r);
      expectPoly(bq, a);
    });

    test('degree(remainder) < degree(divisor)', () {
      final a = [5, 1, 3, 2];
      final b = [2, 1];
      final (_, r) = polynomialDivmod(a, b);
      expect(polynomialDegree(r), lessThan(polynomialDegree(b)));
    });

    test('a has lower degree than b: quotient = [], remainder = a', () {
      final (q, r) = polynomialDivmod([1, 2], [1, 2, 3]);
      expectPoly(q, []);
      expectPoly(r, [1, 2]);
    });

    test('exact division: [6, 11, 6, 1] ÷ [2, 1] (divisible)', () {
      // (x+1)(x+2)(x+3) = (x+1)·(x²+5x+6) = x³+6x²+11x+6
      // Represents [6, 11, 6, 1] in little-endian.
      // Divide by (x+1) = [1, 1]: remainder should be []
      final (q, r) = polynomialDivmod([6, 11, 6, 1], [1, 1]);
      expectPoly(r, []);
      // Verify reconstruction
      expectPoly(polynomialMultiply(q, [1, 1]), [6, 11, 6, 1]);
    });

    test('divide by zero throws ArgumentError', () {
      expect(() => polynomialDivmod([1, 2], []), throwsArgumentError);
      expect(() => polynomialDivmod([1, 2], [0]), throwsArgumentError);
    });
  });

  // ==========================================================================
  // divide and mod (convenience wrappers)
  // ==========================================================================

  group('polynomialDivide and polynomialMod', () {
    test('divide returns quotient', () {
      expectPoly(polynomialDivide([5, 1, 3, 2], [2, 1]), [3, -1, 2]);
    });

    test('mod returns remainder', () {
      expectPoly(polynomialMod([5, 1, 3, 2], [2, 1]), [-1]);
    });
  });

  // ==========================================================================
  // evaluate
  // ==========================================================================

  group('polynomialEvaluate', () {
    test('evaluate [3,1,2] at x=4 = 39', () {
      // 3 + 4 + 2·16 = 3 + 4 + 32 = 39
      expect(polynomialEvaluate([3, 1, 2], 4), closeTo(39, 1e-10));
    });

    test('evaluate zero polynomial at any x = 0', () {
      expect(polynomialEvaluate([], 5), equals(0));
      expect(polynomialEvaluate([0], 100), equals(0));
    });

    test('evaluate constant polynomial at any x = constant', () {
      expect(polynomialEvaluate([7], 100), closeTo(7, 1e-10));
      expect(polynomialEvaluate([7], 0), closeTo(7, 1e-10));
    });

    test('evaluate at x=0 returns constant term', () {
      expect(polynomialEvaluate([3, 1, 2], 0), closeTo(3, 1e-10));
    });

    test('evaluate at x=1 returns sum of coefficients', () {
      // p(1) = sum of all coefficients
      expect(polynomialEvaluate([1, 2, 3], 1), closeTo(6, 1e-10));
    });

    test('Horner is equivalent to naive evaluation', () {
      final p = [5, 0, -3, 1]; // 5 - 3x² + x³
      // Naive: 5 + 0*2 - 3*4 + 1*8 = 5 + 0 - 12 + 8 = 1
      expect(polynomialEvaluate(p, 2), closeTo(1, 1e-10));
    });
  });

  // ==========================================================================
  // gcd
  // ==========================================================================

  group('polynomialGcd', () {
    test('gcd(p, []) = p (GCD with zero is the other polynomial)', () {
      expectPoly(polynomialGcd([1, 2, 1], []), [1, 2, 1]);
    });

    test('gcd([], p) = p', () {
      expectPoly(polynomialGcd([], [1, 2, 1]), [1, 2, 1]);
    });

    test('gcd(p, p) = p (normalized)', () {
      final p = [2, 3, 1];
      expectPoly(polynomialGcd(p, p), p);
    });

    test('gcd of coprime polynomials is constant', () {
      // [6,7,1] = (x+1)(x+6) and [6,5,1] = (x+2)(x+3) share no common root.
      // GCD should be a constant (degree 0).
      final g = polynomialGcd([6, 7, 1], [6, 5, 1]);
      // We don't assert exact value (depends on normalization); just that
      // the GCD divides both with zero remainder.
      expect(polynomialDegree(polynomialMod([6, 7, 1], g)), lessThan(0));
      expect(polynomialDegree(polynomialMod([6, 5, 1], g)), lessThan(0));
    });

    test('gcd divides both inputs', () {
      // (x+1)(x+2) = [2, 3, 1], (x+1)(x+3) = [3, 4, 1]
      // GCD should be proportional to (x+1) = [1, 1]
      final a = [2, 3, 1];  // (x+1)(x+2)
      final b = [3, 4, 1];  // (x+1)(x+3)
      final g = polynomialGcd(a, b);
      // GCD divides both
      expect(polynomialDegree(polynomialMod(a, g)), lessThan(0));
      expect(polynomialDegree(polynomialMod(b, g)), lessThan(0));
    });
  });

  // ==========================================================================
  // Edge cases
  // ==========================================================================

  group('edge cases', () {
    test('all operations with zero polynomial', () {
      expectPoly(polynomialAdd([], []), []);
      expectPoly(polynomialSubtract([], []), []);
      expectPoly(polynomialMultiply([], []), []);
      expect(polynomialEvaluate([], 42), equals(0));
    });

    test('add two big polynomials', () {
      final a = List<num>.generate(100, (i) => i.toDouble());
      final b = List<num>.generate(100, (i) => (100 - i).toDouble());
      final sum = polynomialAdd(a, b);
      // Every coefficient should be 100
      for (int i = 0; i < 100; i++) {
        expect(sum[i], closeTo(100, 1e-10), reason: 'sum[$i] should be 100');
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
