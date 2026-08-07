import 'dart:math' as math;

import 'package:coding_adventures_trig/trig.dart' as trig;
import 'package:test/test.dart';

const tolerance = 1e-10;

Matcher closeToExpected(double expected) => closeTo(expected, tolerance);

void main() {
  group('angle constants', () {
    test('provide a shared full and quarter rotation', () {
      expect(trig.pi, 3.141592653589793);
      expect(trig.twoPi, trig.pi * 2.0);
      expect(trig.halfPi, trig.pi / 2.0);
    });
  });

  group('sin and cos', () {
    test('match the PHY00 special angles', () {
      final sineCases = <(double, double)>[
        (0.0, 0.0),
        (trig.pi / 6.0, 0.5),
        (trig.pi / 4.0, 0.7071067811865476),
        (trig.pi / 3.0, 0.8660254037844386),
        (trig.halfPi, 1.0),
        (trig.pi, 0.0),
        (3.0 * trig.halfPi, -1.0),
        (trig.twoPi, 0.0),
      ];
      final cosineCases = <(double, double)>[
        (0.0, 1.0),
        (trig.pi / 6.0, 0.8660254037844386),
        (trig.pi / 4.0, 0.7071067811865476),
        (trig.pi / 3.0, 0.5),
        (trig.halfPi, 0.0),
        (trig.pi, -1.0),
        (3.0 * trig.halfPi, 0.0),
        (trig.twoPi, 1.0),
      ];

      for (final (angle, expected) in sineCases) {
        expect(trig.sin(angle), closeToExpected(expected));
      }
      for (final (angle, expected) in cosineCases) {
        expect(trig.cos(angle), closeToExpected(expected));
      }
    });

    test('preserve symmetry and the Pythagorean identity', () {
      for (final angle in [0.0, 0.5, 1.0, 1.5, 2.7, -2.5, 5.5]) {
        expect(trig.sin(-angle), closeToExpected(-trig.sin(angle)));
        expect(trig.cos(-angle), closeToExpected(trig.cos(angle)));
        final sine = trig.sin(angle);
        final cosine = trig.cos(angle);
        expect(sine * sine + cosine * cosine, closeToExpected(1.0));
      }
    });

    test('range-reduce large positive and negative inputs', () {
      expect(trig.sin(1000.0 * trig.pi), closeToExpected(0.0));
      expect(trig.cos(1000.0 * trig.pi), closeToExpected(1.0));
      expect(trig.sin(-100.0), closeToExpected(-trig.sin(100.0)));
    });

    test('meet the PHY00 finite-range precision target', () {
      for (final angle in [
        -1e6,
        -12345.6789,
        -100.0,
        -3.25,
        -0.001,
        0.001,
        3.25,
        100.0,
        12345.6789,
        1e6,
      ]) {
        expect(trig.sin(angle), closeToExpected(math.sin(angle)));
        expect(trig.cos(angle), closeToExpected(math.cos(angle)));
        expect(trig.atan(angle), closeToExpected(math.atan(angle)));
      }
    });
  });

  group('angle conversion', () {
    test('converts and round-trips degrees and radians', () {
      expect(trig.radians(180.0), closeToExpected(trig.pi));
      expect(trig.radians(90.0), closeToExpected(trig.halfPi));
      expect(trig.degrees(trig.pi), closeToExpected(180.0));
      expect(trig.degrees(trig.halfPi), closeToExpected(90.0));

      for (final degrees in [0.0, 30.0, 45.0, 90.0, 180.0, 360.0]) {
        expect(trig.degrees(trig.radians(degrees)), closeToExpected(degrees));
      }
    });
  });

  group('sqrt', () {
    test('uses the PHY00 Newton convergence contract', () {
      for (final (value, expected) in <(double, double)>[
        (0.0, 0.0),
        (1.0, 1.0),
        (4.0, 2.0),
        (9.0, 3.0),
        (2.0, 1.41421356237),
        (0.25, 0.5),
        (1e10, 1e5),
      ]) {
        expect(trig.sqrt(value), closeToExpected(expected));
      }
    });

    test('rejects negative input', () {
      expect(() => trig.sqrt(-1.0), throwsArgumentError);
    });

    test('covers tiny, subnormal, infinite, and signed-zero inputs', () {
      final tiny = trig.sqrt(1e-100);
      expect((tiny - 1e-50).abs() / 1e-50, lessThan(1e-12));

      final subnormalExpected = math.sqrt(double.minPositive);
      final subnormalActual = trig.sqrt(double.minPositive);
      expect(
        (subnormalActual - subnormalExpected).abs() / subnormalExpected,
        lessThan(1e-12),
      );

      expect(trig.sqrt(double.infinity), double.infinity);
      expect(trig.sqrt(double.nan).isNaN, isTrue);
      expect(trig.sqrt(-0.0).isNegative, isTrue);
    });
  });

  group('tan, atan, and atan2', () {
    test('computes tangent and signed finite pole sentinels', () {
      expect(trig.tan(0.0), closeToExpected(0.0));
      expect(trig.tan(trig.pi / 4.0), closeToExpected(1.0));
      expect(trig.tan(-trig.pi / 4.0), closeToExpected(-1.0));
      expect(trig.tan(trig.halfPi), 1e308);
      expect(trig.tan(-trig.halfPi), -1e308);
    });

    test('covers arctangent reductions and inversion', () {
      final rootThree = trig.sqrt(3.0);
      expect(trig.atan(0.0), closeToExpected(0.0));
      expect(trig.atan(1.0), closeToExpected(trig.pi / 4.0));
      expect(trig.atan(-1.0), closeToExpected(-trig.pi / 4.0));
      expect(trig.atan(rootThree), closeToExpected(trig.pi / 3.0));
      expect(trig.atan(1.0 / rootThree), closeToExpected(trig.pi / 6.0));
      expect(
        trig.atan(trig.tan(trig.pi / 4.0)),
        closeToExpected(trig.pi / 4.0),
      );
      expect(trig.atan(1e10), closeTo(trig.halfPi, 1e-5));
      expect(trig.atan(-1e10), closeTo(-trig.halfPi, 1e-5));
    });

    test('preserves tiny inputs and negative zero exactly', () {
      expect(trig.atan(-0.0).isNegative, isTrue);
      expect(trig.atan(1.0 / 1073741824.0), 1.0 / 1073741824.0);
      expect(trig.atan(double.minPositive), double.minPositive);
      expect(trig.atan(-double.minPositive), -double.minPositive);
    });

    test('selects both axes, the origin, and every quadrant', () {
      for (final (y, x, expected) in <(double, double, double)>[
        (0.0, 1.0, 0.0),
        (1.0, 0.0, trig.halfPi),
        (0.0, -1.0, trig.pi),
        (-1.0, 0.0, -trig.halfPi),
        (0.0, 0.0, 0.0),
        (1.0, 1.0, trig.pi / 4.0),
        (1.0, -1.0, 3.0 * trig.pi / 4.0),
        (-1.0, -1.0, -3.0 * trig.pi / 4.0),
        (-1.0, 1.0, -trig.pi / 4.0),
      ]) {
        expect(trig.atan2(y, x), closeToExpected(expected));
      }
    });
  });
}
