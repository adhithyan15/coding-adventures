import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:coding_adventures_bezier2d/bezier2d.dart';
import 'package:coding_adventures_point2d/point2d.dart';
import 'package:test/test.dart';

const epsilon = 1e-12;

Point point(List<dynamic> pair) =>
    Point((pair[0] as num).toDouble(), (pair[1] as num).toDouble());

Rect rect(List<dynamic> values) => Rect(
  (values[0] as num).toDouble(),
  (values[1] as num).toDouble(),
  (values[2] as num).toDouble(),
  (values[3] as num).toDouble(),
);

QuadraticBezier quadratic(List<dynamic> controls) => QuadraticBezier(
  point(controls[0] as List<dynamic>),
  point(controls[1] as List<dynamic>),
  point(controls[2] as List<dynamic>),
);

CubicBezier cubic(List<dynamic> controls) => CubicBezier(
  point(controls[0] as List<dynamic>),
  point(controls[1] as List<dynamic>),
  point(controls[2] as List<dynamic>),
  point(controls[3] as List<dynamic>),
);

void nearPoint(Point expected, Point actual) {
  expect(actual.x, closeTo(expected.x, epsilon));
  expect(actual.y, closeTo(expected.y, epsilon));
}

void nearRect(Rect expected, Rect actual) {
  expect(actual.x, closeTo(expected.x, epsilon));
  expect(actual.y, closeTo(expected.y, epsilon));
  expect(actual.width, closeTo(expected.width, epsilon));
  expect(actual.height, closeTo(expected.height, epsilon));
}

double segmentDistance(Point p, Point a, Point b) {
  final chord = b.subtract(a);
  final length2 = chord.magnitudeSquared();
  final t = length2 == 0
      ? 0.0
      : (p.subtract(a).dot(chord) / length2).clamp(0.0, 1.0).toDouble();
  return p.distance(a.add(chord.scale(t)));
}

double polylineDistance(Point p, List<Point> line) {
  var best = double.infinity;
  for (var i = 1; i < line.length; i++) {
    best = math.min(best, segmentDistance(p, line[i - 1], line[i]));
  }
  return best;
}

void main() {
  test('consumes exactly the three G2D02 polynomial cases', () {
    final corpus =
        jsonDecode(
              File(
                '../../../specs/fixtures/geometry2d-v1/cases.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    expect(corpus['version'], 1);
    expect(corpus['absolute_tolerance'], epsilon);
    final seen = <String>{};
    for (final raw in corpus['cases'] as List<dynamic>) {
      final fixture = raw as Map<String, dynamic>;
      final operation = fixture['operation'] as String;
      if (!operation.startsWith('bezier-')) continue;
      final id = fixture['id'] as String;
      expect(seen.add(id), isTrue, reason: 'duplicate fixture $id');
      final controls = fixture['control_points'] as List<dynamic>;
      final split = fixture['expected_split'] as List<dynamic>;
      final t = (fixture['t'] as num).toDouble();
      switch (operation) {
        case 'bezier-quadratic':
          final curve = quadratic(controls);
          nearPoint(
            point(fixture['expected_point'] as List<dynamic>),
            curve.evaluate(t),
          );
          nearPoint(
            point(fixture['expected_derivative'] as List<dynamic>),
            curve.derivative(t),
          );
          final halves = curve.split(t);
          final left = quadratic(split[0] as List<dynamic>);
          final right = quadratic(split[1] as List<dynamic>);
          nearPoint(left.p0, halves.$1.p0);
          nearPoint(left.p1, halves.$1.p1);
          nearPoint(left.p2, halves.$1.p2);
          nearPoint(right.p0, halves.$2.p0);
          nearPoint(right.p1, halves.$2.p1);
          nearPoint(right.p2, halves.$2.p2);
          nearRect(
            rect(fixture['expected_bounds'] as List<dynamic>),
            curve.boundingBox(),
          );
          nearPoint(curve.evaluate(t), curve.elevate().evaluate(t));
          break;
        case 'bezier-cubic':
          final curve = cubic(controls);
          nearPoint(
            point(fixture['expected_point'] as List<dynamic>),
            curve.evaluate(t),
          );
          nearPoint(
            point(fixture['expected_derivative'] as List<dynamic>),
            curve.derivative(t),
          );
          final halves = curve.split(t);
          final left = cubic(split[0] as List<dynamic>);
          final right = cubic(split[1] as List<dynamic>);
          nearPoint(left.p0, halves.$1.p0);
          nearPoint(left.p1, halves.$1.p1);
          nearPoint(left.p2, halves.$1.p2);
          nearPoint(left.p3, halves.$1.p3);
          nearPoint(right.p0, halves.$2.p0);
          nearPoint(right.p1, halves.$2.p1);
          nearPoint(right.p2, halves.$2.p2);
          nearPoint(right.p3, halves.$2.p3);
          nearRect(
            rect(fixture['expected_bounds'] as List<dynamic>),
            curve.boundingBox(),
          );
          break;
        default:
          fail('unknown Bezier operation: $operation');
      }
    }
    expect(seen, {
      'bezier-quadratic-quarter',
      'bezier-cubic-quarter',
      'bezier-cubic-x-overshoot',
    });
  });

  test('consumes all nine safe-flattening cases', () {
    final corpus =
        jsonDecode(
              File(
                '../../../specs/fixtures/bezier2d-flattening-v1/cases.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    expect(corpus['version'], 1);
    expect(corpus['absolute_tolerance'], epsilon);
    expect(corpus['max_depth'], 32);
    expect(corpus['max_subdivisions'], 65535);
    final seen = <String>{};
    for (final raw in corpus['cases'] as List<dynamic>) {
      final fixture = raw as Map<String, dynamic>;
      final id = fixture['id'] as String;
      expect(seen.add(id), isTrue, reason: 'duplicate fixture $id');
      final controls = fixture['control_points'] as List<dynamic>;
      final tolerance = (fixture['tolerance'] as num).toDouble();
      final isQuadratic = fixture['degree'] == 2;
      List<Point> flatten() => isQuadratic
          ? quadratic(controls).toPolyline(tolerance)
          : cubic(controls).toPolyline(tolerance);
      final disposition = fixture['expected_disposition'] as String;
      if (disposition == 'invalid-tolerance') {
        expect(flatten, throwsArgumentError, reason: id);
        continue;
      }
      if (fixture['expected_termination'] == 'budget-error') {
        expect(flatten, throwsStateError, reason: id);
        continue;
      }
      final line = flatten();
      nearPoint(point(controls.first as List<dynamic>), line.first);
      nearPoint(point(controls.last as List<dynamic>), line.last);
      if (disposition == 'chord') {
        expect(line, hasLength(2), reason: id);
      } else if (disposition == 'subdivide') {
        expect(line.length, greaterThan(2), reason: id);
      } else {
        fail('unknown flattening disposition: $disposition');
      }
      if (fixture.containsKey('witness_t')) {
        final t = (fixture['witness_t'] as num).toDouble();
        final witness = isQuadratic
            ? quadratic(controls).evaluate(t)
            : cubic(controls).evaluate(t);
        nearPoint(point(fixture['expected_witness'] as List<dynamic>), witness);
        expect(
          polylineDistance(witness, line),
          lessThanOrEqualTo(tolerance + epsilon),
          reason: id,
        );
      }
    }
    expect(seen, {
      'quadratic-straight',
      'quadratic-arch',
      'cubic-collinear-inside',
      'cubic-symmetric-s',
      'cubic-collinear-overshoot',
      'cubic-coincident-loop',
      'zero-tolerance',
      'negative-tolerance',
      'quadratic-budget-exhaustion',
    });
  });

  test('rejects nonfinite control points and tolerance', () {
    final q = QuadraticBezier(
      const Point(double.nan, 0),
      const Point(1, 1),
      const Point(2, 0),
    );
    expect(() => q.toPolyline(0.1), throwsArgumentError);
    final c = CubicBezier(
      const Point(0, 0),
      const Point(1, 1),
      const Point(2, 1),
      const Point(3, 0),
    );
    expect(() => c.toPolyline(double.infinity), throwsArgumentError);
  });
}
