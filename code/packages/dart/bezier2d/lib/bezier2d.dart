/// G2D02: quadratic and cubic Bezier geometry with bounded safe flattening.
library coding_adventures_bezier2d;

import 'dart:math' as math;

import 'package:coding_adventures_point2d/point2d.dart';

void _requireFinite(Point point) {
  if (!point.x.isFinite || !point.y.isFinite) {
    throw ArgumentError('Bezier control/intermediate point is nonfinite');
  }
}

void _validate(double tolerance, List<Point> controls) {
  if (!tolerance.isFinite || tolerance <= 0) {
    throw ArgumentError('tolerance must be finite and positive');
  }
  for (final point in controls) {
    _requireFinite(point);
  }
}

/// The closest point must lie on the *finite* chord, not its infinite line.
double _segmentDistance(Point control, Point start, Point end) {
  final chord = end.subtract(start);
  final lengthSquared = chord.magnitudeSquared();
  final u = lengthSquared == 0
      ? 0.0
      : (control.subtract(start).dot(chord) / lengthSquared)
          .clamp(0.0, 1.0)
          .toDouble();
  final distance = control.distance(start.add(chord.scale(u)));
  if (!distance.isFinite) throw ArgumentError('Bezier flatness overflowed');
  return distance;
}

Rect _bounds(List<Point> candidates) {
  var minX = double.infinity;
  var minY = double.infinity;
  var maxX = double.negativeInfinity;
  var maxY = double.negativeInfinity;
  for (final point in candidates) {
    minX = math.min(minX, point.x);
    minY = math.min(minY, point.y);
    maxX = math.max(maxX, point.x);
    maxY = math.max(maxY, point.y);
  }
  return Rect(minX, minY, maxX - minX, maxY - minY);
}

/// Solve one cubic coordinate's derivative, falling back to the linear case.
List<double> _cubicDerivativeRoots(double p0, double p1, double p2, double p3) {
  final alpha = p1 - p0;
  final beta = 2 * (p0 - 2 * p1 + p2);
  final gamma = -p0 + 3 * p1 - 3 * p2 + p3;
  if (gamma.abs() < 1e-12) {
    if (beta.abs() < 1e-12) return [];
    return [-alpha / beta];
  }
  final discriminant = beta * beta - 4 * gamma * alpha;
  if (discriminant < 0) return [];
  final root = math.sqrt(discriminant);
  return [(-beta - root) / (2 * gamma), (-beta + root) / (2 * gamma)];
}

/// A three-control-point quadratic Bezier segment.
final class QuadraticBezier {
  final Point p0;
  final Point p1;
  final Point p2;

  const QuadraticBezier(this.p0, this.p1, this.p2);

  Point evaluate(double t) => p0.lerp(p1, t).lerp(p1.lerp(p2, t), t);

  Point derivative(double t) =>
      p1.subtract(p0).scale(1 - t).add(p2.subtract(p1).scale(t)).scale(2);

  (QuadraticBezier, QuadraticBezier) split(double t) {
    final q0 = p0.lerp(p1, t);
    final q1 = p1.lerp(p2, t);
    final middle = q0.lerp(q1, t);
    return (QuadraticBezier(p0, q0, middle), QuadraticBezier(middle, q1, p2));
  }

  CubicBezier elevate() => CubicBezier(
        p0,
        p0.scale(1 / 3).add(p1.scale(2 / 3)),
        p1.scale(2 / 3).add(p2.scale(1 / 3)),
        p2,
      );

  Rect boundingBox() {
    final candidates = <Point>[p0, p2];
    final xDenominator = p0.x - 2 * p1.x + p2.x;
    final yDenominator = p0.y - 2 * p1.y + p2.y;
    if (xDenominator.abs() >= 1e-12) {
      final t = (p0.x - p1.x) / xDenominator;
      if (t > 0 && t < 1) candidates.add(evaluate(t));
    }
    if (yDenominator.abs() >= 1e-12) {
      final t = (p0.y - p1.y) / yDenominator;
      if (t > 0 && t < 1) candidates.add(evaluate(t));
    }
    return _bounds(candidates);
  }

  List<Point> toPolyline(double tolerance) {
    _validate(tolerance, [p0, p1, p2]);
    final points = <Point>[p0];
    var subdivisions = 0;

    void visit(QuadraticBezier curve, int depth) {
      final error = _segmentDistance(curve.p1, curve.p0, curve.p2);
      if (error <= tolerance) {
        points.add(curve.p2);
        return;
      }
      if (depth == 32 || subdivisions == 65535) {
        throw StateError('Bezier subdivision budget exhausted');
      }
      subdivisions++;
      final (left, right) = curve.split(0.5);
      _requireFinite(left.p1);
      _requireFinite(left.p2);
      _requireFinite(right.p1);
      visit(left, depth + 1);
      visit(right, depth + 1);
    }

    // A failure throws before this return, never exposing the partial list.
    visit(this, 0);
    return List<Point>.unmodifiable(points);
  }
}

/// A four-control-point cubic Bezier segment.
final class CubicBezier {
  final Point p0;
  final Point p1;
  final Point p2;
  final Point p3;

  const CubicBezier(this.p0, this.p1, this.p2, this.p3);

  Point evaluate(double t) {
    final q0 = p0.lerp(p1, t);
    final q1 = p1.lerp(p2, t);
    final q2 = p2.lerp(p3, t);
    return q0.lerp(q1, t).lerp(q1.lerp(q2, t), t);
  }

  Point derivative(double t) {
    final complement = 1 - t;
    return p1
        .subtract(p0)
        .scale(complement * complement)
        .add(p2.subtract(p1).scale(2 * complement * t))
        .add(p3.subtract(p2).scale(t * t))
        .scale(3);
  }

  (CubicBezier, CubicBezier) split(double t) {
    final q0 = p0.lerp(p1, t);
    final q1 = p1.lerp(p2, t);
    final q2 = p2.lerp(p3, t);
    final r0 = q0.lerp(q1, t);
    final r1 = q1.lerp(q2, t);
    final middle = r0.lerp(r1, t);
    return (CubicBezier(p0, q0, r0, middle), CubicBezier(middle, r1, q2, p3));
  }

  Rect boundingBox() {
    final candidates = <Point>[p0, p3];
    for (final t in _cubicDerivativeRoots(p0.x, p1.x, p2.x, p3.x)) {
      if (t > 0 && t < 1) candidates.add(evaluate(t));
    }
    for (final t in _cubicDerivativeRoots(p0.y, p1.y, p2.y, p3.y)) {
      if (t > 0 && t < 1) candidates.add(evaluate(t));
    }
    return _bounds(candidates);
  }

  List<Point> toPolyline(double tolerance) {
    _validate(tolerance, [p0, p1, p2, p3]);
    final points = <Point>[p0];
    var subdivisions = 0;

    void visit(CubicBezier curve, int depth) {
      // The convex-hull distance to the finite chord bounds the whole curve.
      // A midpoint sample misses the symmetric S-curve counterexample.
      final error = math.max(
        _segmentDistance(curve.p1, curve.p0, curve.p3),
        _segmentDistance(curve.p2, curve.p0, curve.p3),
      );
      if (error <= tolerance) {
        points.add(curve.p3);
        return;
      }
      if (depth == 32 || subdivisions == 65535) {
        throw StateError('Bezier subdivision budget exhausted');
      }
      subdivisions++;
      final (left, right) = curve.split(0.5);
      _requireFinite(left.p1);
      _requireFinite(left.p2);
      _requireFinite(left.p3);
      _requireFinite(right.p1);
      _requireFinite(right.p2);
      visit(left, depth + 1);
      visit(right, depth + 1);
    }

    visit(this, 0);
    return List<Point>.unmodifiable(points);
  }
}
