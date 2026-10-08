import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:coding_adventures_arc2d/arc2d.dart';
import 'package:coding_adventures_point2d/point2d.dart';
import 'package:test/test.dart';

Point point(List<dynamic> value) =>
    Point((value[0] as num).toDouble(), (value[1] as num).toDouble());

void close(double actual, double expected) =>
    expect(actual, closeTo(expected, 1e-12));

void closePoint(Point actual, Point expected) {
  close(actual.x, expected.x);
  close(actual.y, expected.y);
}

void closeRect(Rect actual, List<dynamic> expected) {
  close(actual.x, (expected[0] as num).toDouble());
  close(actual.y, (expected[1] as num).toDouble());
  close(actual.width, (expected[2] as num).toDouble());
  close(actual.height, (expected[3] as num).toDouble());
}

void main() {
  test('all neutral Arc2D cases', () {
    final corpus = jsonDecode(
        File('../../../specs/fixtures/geometry2d-v1/cases.json')
            .readAsStringSync()) as Map<String, dynamic>;
    var consumed = 0;
    for (final fixture in corpus['cases'] as List<dynamic>) {
      final entry = fixture as Map<String, dynamic>;
      final operation = entry['operation'];
      if (operation == 'svg-arc-degenerate') {
        final arc = SvgArc(
            point(entry['from'] as List<dynamic>),
            point(entry['to'] as List<dynamic>),
            (entry['rx'] as num).toDouble(),
            (entry['ry'] as num).toDouble(),
            0,
            false,
            true);
        expect(arc.toCenterArc(), isNull, reason: entry['id'] as String);
        expect(arc.toCubicBeziers(), isEmpty);
        closePoint(arc.evaluate((entry['sample_t'] as num).toDouble()),
            point(entry['expected_point'] as List<dynamic>));
        closeRect(arc.boundingBox(), entry['expected_bounds'] as List<dynamic>);
        consumed++;
      } else if (operation == 'center-arc-bounds' ||
          operation == 'center-arc-cubics') {
        final arc = CenterArc(
            point(entry['center'] as List<dynamic>),
            (entry['rx'] as num).toDouble(),
            (entry['ry'] as num).toDouble(),
            (entry['start_angle'] as num).toDouble(),
            (entry['sweep_angle'] as num).toDouble(),
            (entry['x_rotation'] as num).toDouble());
        if (operation == 'center-arc-bounds') {
          closeRect(
              arc.boundingBox(), entry['expected_bounds'] as List<dynamic>);
        } else if (entry.containsKey('expected_error')) {
          expect(() => arc.toCubicBeziers(), throwsArgumentError);
          expect(() => arc.boundingBox(), throwsArgumentError);
          expect(() => arc.evaluate(0), throwsArgumentError);
        } else {
          final cubics = arc.toCubicBeziers();
          expect(cubics.length, entry['expected_count']);
          closePoint(cubics.first.p0, arc.evaluate(0));
          closePoint(cubics.last.p3, arc.evaluate(1));
          for (var i = 1; i < cubics.length; i++) {
            closePoint(cubics[i - 1].p3, cubics[i].p0);
          }
          if (arc.sweepAngle == 0) {
            closePoint(cubics[0].p0, cubics[0].p1);
            closePoint(cubics[0].p0, cubics[0].p2);
            closePoint(cubics[0].p0, cubics[0].p3);
          }
        }
        consumed++;
      }
    }
    expect(consumed, 11);
  });

  test('SVG W3C conversion, tangent, and cubic approximation', () {
    final quarter =
        SvgArc(const Point(1, 0), const Point(0, 1), 1, 1, 0, false, true);
    final center = quarter.toCenterArc()!;
    closePoint(center.center, const Point.origin());
    close(center.startAngle, 0);
    close(center.sweepAngle, math.pi / 2);
    closePoint(center.evaluate(0), quarter.from);
    closePoint(center.evaluate(1), quarter.to);
    closePoint(center.tangent(0), Point(0, math.pi / 2));
    closePoint(quarter.evaluate(.5), center.evaluate(.5));
    close(quarter.boundingBox().x, 0);
    expect(quarter.toCubicBeziers(), hasLength(1));
    close(quarter.toCubicBeziers()[0].p1.y, 4 / 3 * math.tan(math.pi / 8));

    final half =
        SvgArc(const Point(1, 0), const Point(-1, 0), 1, 1, 0, false, true);
    close(half.toCenterArc()!.sweepAngle, math.pi);
    closePoint(half.evaluate(.5), const Point(0, 1));
    expect(half.toCubicBeziers(), hasLength(2));
    final scaled = SvgArc(
        const Point.origin(), const Point(10, 0), .1, .1, 0, false, true);
    expect(scaled.toCenterArc()!.rx, greaterThanOrEqualTo(5));
  });

  test('large/sweep flags, rotation, signed radii and strict thresholds', () {
    const from = Point(1, 0), to = Point(0, 1);
    for (final large in [false, true]) {
      for (final sweep in [false, true]) {
        final arc =
            SvgArc(from, to, 2, 2, math.pi / 4, large, sweep).toCenterArc()!;
        expect(arc.sweepAngle.abs() > math.pi, large);
        expect(arc.sweepAngle > 0, sweep);
        closePoint(arc.evaluate(0), from);
        closePoint(arc.evaluate(1), to);
      }
    }
    for (final arc in [
      SvgArc(const Point.origin(), const Point(1, 0), 1e-10, 1, 0, false, true),
      SvgArc(const Point.origin(), const Point(1e-10, 0), 1, 1, 0, false, true),
      SvgArc(const Point.origin(), const Point(8e-11, 8e-11), 1, 1, 0, false,
          true),
    ]) {
      expect(arc.toCenterArc(), isNotNull);
      expect(arc.toCenterArc()!.center.x.isFinite, isTrue);
    }
    closePoint(SvgArc(from, to, 2, 2, 0, false, true).toCenterArc()!.center,
        SvgArc(from, to, -2, 2, 0, false, true).toCenterArc()!.center);
  });

  test('nonfinite, nonpositive and over-turn direct centers fail closed', () {
    for (final value in [
      double.nan,
      double.infinity,
      double.negativeInfinity
    ]) {
      final arcs = [
        CenterArc(Point(value, 0), 1, 1, 0, 1, 0),
        CenterArc(const Point.origin(), value, 1, 0, 1, 0),
        CenterArc(const Point.origin(), 1, 1, value, 1, 0),
        CenterArc(const Point.origin(), 1, 1, 0, value, 0),
        CenterArc(const Point.origin(), 1, 1, 0, 1, value),
      ];
      for (final arc in arcs)
        expect(() => arc.toCubicBeziers(), throwsArgumentError);
    }
    for (final radius in [0.0, -1.0]) {
      expect(
          () =>
              CenterArc(const Point.origin(), radius, 1, 0, 1, 0).boundingBox(),
          throwsArgumentError);
    }
    expect(
        () => CenterArc(
                const Point(double.maxFinite, 0), double.maxFinite, 1, 0, 1, 0)
            .toCubicBeziers(),
        throwsArgumentError);
  });

  test('degenerate line rejects nonfinite results', () {
    final line = SvgArc(
        const Point(-1e308, 0), const Point(1e308, 0), 0, 1, 0, false, true);
    expect(() => line.boundingBox(), throwsArgumentError);
    expect(() => line.evaluate(.5), throwsArgumentError);
    final ordinary =
        SvgArc(const Point.origin(), const Point(1, 1), 0, 1, 0, false, true);
    expect(() => ordinary.evaluate(double.nan), throwsArgumentError);
    expect(() => ordinary.evaluate(double.infinity), throwsArgumentError);
  });
}
