/// G2D01: immutable two-dimensional affine matrices in SVG scalar order.
library coding_adventures_affine2d;

import 'package:coding_adventures_point2d/point2d.dart';
import 'package:coding_adventures_trig/trig.dart' as trig;

/// The implicit third row is [0,0,1]. A point has third coordinate one;
/// a direction has zero, so [applyToVector] deliberately omits translation.
final class Affine2D {
  final double a;
  final double b;
  final double c;
  final double d;
  final double e;
  final double f;

  const Affine2D(this.a, this.b, this.c, this.d, this.e, this.f);

  const Affine2D.identity() : this(1, 0, 0, 1, 0, 0);

  static Affine2D translate(double tx, double ty) =>
      Affine2D(1, 0, 0, 1, tx, ty);

  static Affine2D rotate(double angle) {
    final sine = trig.sin(angle);
    final cosine = trig.cos(angle);
    return Affine2D(cosine, sine, -sine, cosine, 0, 0);
  }

  /// Conjugating an origin rotation keeps [center] in place.
  static Affine2D rotateAround(Point center, double angle) => translate(
    center.x,
    center.y,
  ).multiply(rotate(angle)).multiply(translate(-center.x, -center.y));

  static Affine2D scale(double sx, double sy) => Affine2D(sx, 0, 0, sy, 0, 0);
  static Affine2D scaleUniform(double factor) => scale(factor, factor);
  static Affine2D skewX(double angle) =>
      Affine2D(1, 0, trig.tan(angle), 1, 0, 0);
  static Affine2D skewY(double angle) =>
      Affine2D(1, trig.tan(angle), 0, 1, 0, 0);

  /// Matrix multiplication applies [other] first, then this transform.
  Affine2D multiply(Affine2D other) => Affine2D(
    a * other.a + c * other.b,
    b * other.a + d * other.b,
    a * other.c + c * other.d,
    b * other.c + d * other.d,
    a * other.e + c * other.f + e,
    b * other.e + d * other.f + f,
  );

  Point applyToPoint(Point point) =>
      Point(a * point.x + c * point.y + e, b * point.x + d * point.y + f);

  Point applyToVector(Point vector) =>
      Point(a * vector.x + c * vector.y, b * vector.x + d * vector.y);

  double determinant() => a * d - b * c;

  /// A near-zero area scale cannot be inverted numerically.
  Affine2D? invert() {
    final det = determinant();
    if (det.abs() < 1e-12) return null;
    final reciprocal = 1 / det;
    return Affine2D(
      d * reciprocal,
      -b * reciprocal,
      -c * reciprocal,
      a * reciprocal,
      (c * f - d * e) * reciprocal,
      (b * e - a * f) * reciprocal,
    );
  }

  bool isIdentity() =>
      (a - 1).abs() < 1e-10 &&
      b.abs() < 1e-10 &&
      c.abs() < 1e-10 &&
      (d - 1).abs() < 1e-10 &&
      e.abs() < 1e-10 &&
      f.abs() < 1e-10;

  bool isTranslationOnly() =>
      (a - 1).abs() < 1e-10 &&
      b.abs() < 1e-10 &&
      c.abs() < 1e-10 &&
      (d - 1).abs() < 1e-10;

  List<double> toArray() => [a, b, c, d, e, f];

  @override
  bool operator ==(Object other) =>
      other is Affine2D &&
      a == other.a &&
      b == other.b &&
      c == other.c &&
      d == other.d &&
      e == other.e &&
      f == other.f;

  @override
  int get hashCode => Object.hash(a, b, c, d, e, f);
}
