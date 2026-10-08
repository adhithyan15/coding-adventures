/// G2D00: immutable Cartesian points/vectors and half-open bounding boxes.
library coding_adventures_point2d;

import 'dart:math' as math;

import 'package:coding_adventures_trig/trig.dart' as trig;

/// A position from the origin or a displacement between two positions.
///
/// The two meanings share exactly the same pair of coordinates. Returning
/// fresh values from operations means translating a point cannot silently
/// change a vector that another caller still holds.
final class Point {
  final double x;
  final double y;

  const Point(this.x, this.y);
  const Point.origin() : x = 0.0, y = 0.0;

  Point add(Point other) => Point(x + other.x, y + other.y);
  Point subtract(Point other) => Point(x - other.x, y - other.y);
  Point scale(double scalar) => Point(x * scalar, y * scalar);
  Point negate() => Point(-x, -y);

  /// Projection numerator; zero means perpendicular vectors.
  double dot(Point other) => x * other.x + y * other.y;

  /// Signed orientation; positive is a counterclockwise turn.
  double cross(Point other) => x * other.y - y * other.x;

  double magnitudeSquared() => x * x + y * y;
  double magnitude() => math.sqrt(magnitudeSquared());

  /// A magnitude strictly below 1e-12 has no stable direction.
  /// Exactly 1e-12 still normalizes; compare lengths, not squared lengths.
  Point normalize() {
    final length = magnitude();
    return length < 1e-12
        ? const Point.origin()
        : Point(x / length, y / length);
  }

  double distanceSquared(Point other) => subtract(other).magnitudeSquared();
  double distance(Point other) => subtract(other).magnitude();

  /// A + t(B - A), including extrapolation outside the unit interval.
  Point lerp(Point other, double t) => add(other.subtract(this).scale(t));

  Point perpendicular() => Point(-y, x);

  /// Quadrant-sensitive angle through this lane's PHY00 package.
  double angle() => trig.atan2(y, x);

  @override
  bool operator ==(Object other) =>
      other is Point && x == other.x && y == other.y;

  @override
  int get hashCode => Object.hash(x, y);
}

/// An axis-aligned box covering [x, x + width) by [y, y + height).
///
/// Excluding the bottom/right edges avoids double-counting a point shared
/// by adjacent rectangles in a tiled image.
final class Rect {
  final double x;
  final double y;
  final double width;
  final double height;

  const Rect(this.x, this.y, this.width, this.height);
  const Rect.zero() : x = 0.0, y = 0.0, width = 0.0, height = 0.0;

  factory Rect.fromPoints(Point min, Point max) =>
      Rect(min.x, min.y, max.x - min.x, max.y - min.y);

  Point min() => Point(x, y);
  Point max() => Point(x + width, y + height);
  Point center() => Point(x + width / 2, y + height / 2);

  bool isEmpty() => width <= 0 || height <= 0;

  bool containsPoint(Point point) =>
      x <= point.x &&
      point.x < x + width &&
      y <= point.y &&
      point.y < y + height;

  /// Empty boxes are identities rather than extra envelope points.
  Rect union(Rect other) {
    if (isEmpty()) return other;
    if (other.isEmpty()) return this;
    final left = math.min(x, other.x);
    final top = math.min(y, other.y);
    final right = math.max(x + width, other.x + other.width);
    final bottom = math.max(y + height, other.y + other.height);
    return Rect(left, top, right - left, bottom - top);
  }

  /// A boundary touch has no area and returns null.
  Rect? intersection(Rect other) {
    final left = math.max(x, other.x);
    final top = math.max(y, other.y);
    final overlapWidth = math.min(x + width, other.x + other.width) - left;
    final overlapHeight = math.min(y + height, other.y + other.height) - top;
    if (overlapWidth <= 0 || overlapHeight <= 0) return null;
    return Rect(left, top, overlapWidth, overlapHeight);
  }

  Rect expandBy(double amount) =>
      Rect(x - amount, y - amount, width + 2 * amount, height + 2 * amount);

  @override
  bool operator ==(Object other) =>
      other is Rect &&
      x == other.x &&
      y == other.y &&
      width == other.width &&
      height == other.height;

  @override
  int get hashCode => Object.hash(x, y, width, height);
}
