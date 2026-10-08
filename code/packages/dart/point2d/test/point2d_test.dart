import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:coding_adventures_point2d/point2d.dart';
import 'package:test/test.dart';

void main() {
  group('G2D00 immutable Point', () {
    test('arithmetic preserves its operands', () {
      const a = Point(1, 2);
      const b = Point(3, 4);
      expect(Point.origin(), const Point(0, 0));
      expect(a.add(b), const Point(4, 6));
      expect(a.subtract(b), const Point(-2, -2));
      expect(a.scale(2), const Point(2, 4));
      expect(a.negate(), const Point(-1, -2));
      expect(a, const Point(1, 2));
      expect(b, const Point(3, 4));
    });

    test('vector geometry and PHY00 angle quadrants', () {
      const right = Point(1, 0);
      const up = Point(0, 1);
      const threeFour = Point(3, 4);
      expect(right.dot(up), 0);
      expect(right.cross(up), 1);
      expect(up.cross(right), -1);
      expect(threeFour.magnitudeSquared(), 25);
      expect(threeFour.magnitude(), closeTo(5, 1e-12));
      expect(Point.origin().distanceSquared(threeFour), 25);
      expect(Point.origin().distance(threeFour), closeTo(5, 1e-12));
      expect(Point.origin().lerp(const Point(10, 10), 0.5), const Point(5, 5));
      expect(
        Point.origin().lerp(const Point(10, 10), -0.5),
        const Point(-5, -5),
      );
      expect(right.perpendicular(), up);
      expect(right.perpendicular().perpendicular(), right.negate());
      expect(right.angle(), closeTo(0, 1e-10));
      expect(up.angle(), closeTo(math.pi / 2, 1e-9));
      expect(const Point(-1, 0).angle(), closeTo(math.pi, 1e-9));
      expect(const Point(0, -1).angle(), closeTo(-math.pi / 2, 1e-9));
    });
  });

  group('G2D00 half-open Rect', () {
    test('construction, bounds, predicates, and set operations', () {
      const a = Rect(0, 0, 10, 10);
      const b = Rect(5, 5, 10, 10);
      expect(Rect.zero(), const Rect(0, 0, 0, 0));
      expect(Rect.fromPoints(const Point(0, 0), const Point(10, 10)), a);
      expect(a.min(), const Point(0, 0));
      expect(a.max(), const Point(10, 10));
      expect(a.center(), const Point(5, 5));
      expect(a.isEmpty(), isFalse);
      expect(Rect.zero().isEmpty(), isTrue);
      expect(const Rect(0, 0, -1, 2).isEmpty(), isTrue);
      expect(const Rect(0, 0, 2, -1).isEmpty(), isTrue);
      expect(a.containsPoint(const Point(0, 0)), isTrue);
      expect(a.containsPoint(const Point(5, 5)), isTrue);
      expect(a.containsPoint(const Point(10, 10)), isFalse);
      expect(a.containsPoint(const Point(-1, 0)), isFalse);
      expect(Rect.zero().containsPoint(Point.origin()), isFalse);
      expect(a.union(b), const Rect(0, 0, 15, 15));
      expect(a.union(Rect.zero()), a);
      expect(Rect.zero().union(a), a);
      expect(a.intersection(b), const Rect(5, 5, 5, 5));
      expect(a.intersection(const Rect(10, 0, 2, 2)), isNull);
      expect(Rect.zero().intersection(a), isNull);
      expect(const Rect(1, 1, 8, 8).expandBy(1), const Rect(0, 0, 10, 10));
    });
  });

  test('consumes all four checked point-normalize fixtures only', () {
    final corpus =
        jsonDecode(
              File(
                '../../../specs/fixtures/geometry2d-v1/cases.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    expect(corpus['version'], 1);
    expect(corpus['absolute_tolerance'], 1e-12);
    final seen = <String>{};
    for (final raw in corpus['cases'] as List<dynamic>) {
      final fixture = raw as Map<String, dynamic>;
      if (fixture['operation'] != 'point-normalize') continue;
      final id = fixture['id'] as String;
      expect(seen.add(id), isTrue, reason: 'duplicate fixture $id');
      final input = fixture['point'] as List<dynamic>;
      final expected = fixture['expected'] as List<dynamic>;
      final actual = Point(
        (input[0] as num).toDouble(),
        (input[1] as num).toDouble(),
      ).normalize();
      final x = (expected[0] as num).toDouble();
      final y = (expected[1] as num).toDouble();
      switch (fixture['comparison']) {
        case 'exact':
          expect(actual.x, x, reason: id);
          expect(actual.y, y, reason: id);
          break;
        case 'absolute':
          expect(actual.x, closeTo(x, 1e-12), reason: id);
          expect(actual.y, closeTo(y, 1e-12), reason: id);
          break;
        default:
          fail('unknown comparison for $id');
      }
    }
    expect(seen, {
      'normalize-zero',
      'normalize-below-epsilon',
      'normalize-at-epsilon',
      'normalize-three-four',
    });
  });
}
