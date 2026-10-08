/// G2D03: bounded elliptical arcs and SVG endpoint-to-center conversion.
library coding_adventures_arc2d;

import 'dart:math' as math;

import 'package:coding_adventures_bezier2d/bezier2d.dart';
import 'package:coding_adventures_point2d/point2d.dart';
import 'package:coding_adventures_trig/trig.dart' as trig;

void _finite(double value) {
  if (!value.isFinite) throw ArgumentError('nonfinite arc geometry');
}

Point _checked(Point point) {
  _finite(point.x);
  _finite(point.y);
  return point;
}

/// An ellipse center, radii, local start/sweep angles, and axis rotation.
final class CenterArc {
  final Point center;
  final double rx;
  final double ry;
  final double startAngle;
  final double sweepAngle;
  final double xRotation;

  const CenterArc(this.center, this.rx, this.ry, this.startAngle,
      this.sweepAngle, this.xRotation);

  void _validate() {
    _checked(center);
    for (final value in [rx, ry, startAngle, sweepAngle, xRotation]) {
      _finite(value);
    }
    if (rx <= 0 || ry <= 0) throw ArgumentError('radii must be positive');
    if (sweepAngle.abs() > 2 * trig.pi) throw ArgumentError('invalid-sweep');
    _finite(startAngle + sweepAngle);
  }

  Point _at(double angle) {
    final cosR = trig.cos(xRotation), sinR = trig.sin(xRotation);
    final xp = rx * trig.cos(angle), yp = ry * trig.sin(angle);
    return _checked(Point(
        cosR * xp - sinR * yp + center.x, sinR * xp + cosR * yp + center.y));
  }

  Point _derivativeAt(double angle) {
    final cosR = trig.cos(xRotation), sinR = trig.sin(xRotation);
    final dx = -rx * trig.sin(angle), dy = ry * trig.cos(angle);
    return _checked(Point(cosR * dx - sinR * dy, sinR * dx + cosR * dy));
  }

  /// Evaluate at normalized parameter [t], allowing extrapolation.
  Point evaluate(double t) {
    _validate();
    _finite(t);
    final angle = startAngle + t * sweepAngle;
    _finite(angle);
    return _at(angle);
  }

  /// Derivative with respect to normalized parameter [t].
  Point tangent(double t) {
    _validate();
    _finite(t);
    final angle = startAngle + t * sweepAngle;
    _finite(angle);
    return _checked(_derivativeAt(angle).scale(sweepAngle));
  }

  double _positiveMod(double value) {
    final residue = value % (2 * trig.pi);
    return residue < 0 ? residue + 2 * trig.pi : residue;
  }

  bool _includes(double angle) {
    if (sweepAngle > 0) return _positiveMod(angle - startAngle) <= sweepAngle;
    if (sweepAngle < 0) return _positiveMod(startAngle - angle) <= -sweepAngle;
    return false;
  }

  /// Analytic extrema with directed periodic membership, not sampling.
  Rect boundingBox() {
    _validate();
    final points = <Point>[_at(startAngle), _at(startAngle + sweepAngle)];
    final cosR = trig.cos(xRotation), sinR = trig.sin(xRotation);
    final x = trig.atan2(-ry * sinR, rx * cosR);
    final y = trig.atan2(ry * cosR, rx * sinR);
    for (final angle in [x, x + trig.pi, y, y + trig.pi]) {
      if (_includes(angle)) points.add(_at(angle));
    }
    var minX = double.infinity, minY = double.infinity;
    var maxX = double.negativeInfinity, maxY = double.negativeInfinity;
    for (final point in points) {
      minX = math.min(minX, point.x);
      minY = math.min(minY, point.y);
      maxX = math.max(maxX, point.x);
      maxY = math.max(maxY, point.y);
    }
    _finite(maxX - minX);
    _finite(maxY - minY);
    return Rect(minX, minY, maxX - minX, maxY - minY);
  }

  /// One to four cubic segments, each spanning at most a quarter turn.
  List<CubicBezier> toCubicBeziers() {
    _validate();
    final count = math.max(1, (sweepAngle.abs() / (trig.pi / 2)).ceil());
    if (count > 4) throw ArgumentError('invalid-sweep');
    final segmentSweep = sweepAngle / count;
    final k = 4 / 3 * trig.tan(segmentSweep / 4);
    _finite(k);
    return List<CubicBezier>.unmodifiable(
        List<CubicBezier>.generate(count, (index) {
      final a = startAngle + index * segmentSweep;
      final b = a + segmentSweep;
      final p0 = _at(a), p3 = _at(b);
      final p1 = _checked(p0.add(_derivativeAt(a).scale(k)));
      final p2 = _checked(p3.subtract(_derivativeAt(b).scale(k)));
      return CubicBezier(p0, p1, p2, p3);
    }));
  }
}

/// SVG `A` command geometry; degenerate endpoint arcs become line segments.
final class SvgArc {
  final Point from;
  final Point to;
  final double rx;
  final double ry;
  final double xRotation;
  final bool largeArc;
  final bool sweep;

  const SvgArc(this.from, this.to, this.rx, this.ry, this.xRotation,
      this.largeArc, this.sweep);

  /// W3C F.6.5 conversion; returns null for zero-radius or coincident arcs.
  CenterArc? toCenterArc() {
    _checked(from);
    _checked(to);
    _finite(rx);
    _finite(ry);
    _finite(xRotation);
    var radiusX = rx.abs(), radiusY = ry.abs();
    if (radiusX < 1e-10 || radiusY < 1e-10 || from.distanceSquared(to) < 1e-20)
      return null;
    final cosR = trig.cos(xRotation), sinR = trig.sin(xRotation);
    final dx = (from.x - to.x) / 2, dy = (from.y - to.y) / 2;
    final x1 = cosR * dx + sinR * dy, y1 = -sinR * dx + cosR * dy;
    _finite(x1);
    _finite(y1);
    var xRatio = x1 / radiusX, yRatio = y1 / radiusY;
    var lambda = xRatio * xRatio + yRatio * yRatio;
    _finite(lambda);
    if (lambda > 1) {
      final scale = trig.sqrt(lambda);
      radiusX *= scale;
      radiusY *= scale;
      xRatio = x1 / radiusX;
      yRatio = y1 / radiusY;
      lambda = xRatio * xRatio + yRatio * yRatio;
    }
    _finite(radiusX);
    _finite(radiusY);
    if (lambda <= 0) throw ArgumentError('unstable SVG center');
    final sign = largeArc != sweep ? 1.0 : -1.0;
    final factor = sign * trig.sqrt(math.max(0, (1 - lambda) / lambda));
    final cxLocal = factor * radiusX * yRatio;
    final cyLocal = -factor * radiusY * xRatio;
    final cx = cosR * cxLocal - sinR * cyLocal + (from.x + to.x) / 2;
    final cy = sinR * cxLocal + cosR * cyLocal + (from.y + to.y) / 2;
    _finite(cx);
    _finite(cy);
    final start =
        trig.atan2((y1 - cyLocal) / radiusY, (x1 - cxLocal) / radiusX);
    final end =
        trig.atan2((-y1 - cyLocal) / radiusY, (-x1 - cxLocal) / radiusX);
    var delta = end - start;
    if (!sweep && delta > 0) delta -= 2 * trig.pi;
    if (sweep && delta < 0) delta += 2 * trig.pi;
    delta = math.max(-2 * trig.pi, math.min(2 * trig.pi, delta));
    _finite(start);
    _finite(delta);
    return CenterArc(Point(cx, cy), radiusX, radiusY, start, delta, xRotation);
  }

  Point evaluate(double t) {
    _finite(t);
    final arc = toCenterArc();
    if (arc != null) return arc.evaluate(t);
    return _checked(from.lerp(to, t));
  }

  Rect boundingBox() {
    final arc = toCenterArc();
    if (arc != null) return arc.boundingBox();
    final bounds = Rect.fromPoints(
      Point(math.min(from.x, to.x), math.min(from.y, to.y)),
      Point(math.max(from.x, to.x), math.max(from.y, to.y)),
    );
    _finite(bounds.width);
    _finite(bounds.height);
    return bounds;
  }

  List<CubicBezier> toCubicBeziers() =>
      toCenterArc()?.toCubicBeziers() ?? const [];
}
