package com.codingadventures.affine2d;

import com.codingadventures.point2d.Point;
import com.codingadventures.trig.Trig;

/**
 * An immutable SVG-order affine matrix {@code [a,b,c,d,e,f]} (G2D01).
 *
 * <p>The implicit last row is {@code [0,0,1]}. A point has homogeneous third
 * coordinate one, while a direction has zero; this is why translation affects
 * {@link #applyToPoint(Point)} but not {@link #applyToVector(Point)}.
 */
public record Affine2D(double a, double b, double c, double d, double e, double f) {
    public static Affine2D identity() {
        return new Affine2D(1, 0, 0, 1, 0, 0);
    }

    public static Affine2D translate(double tx, double ty) {
        return new Affine2D(1, 0, 0, 1, tx, ty);
    }

    public static Affine2D rotate(double angle) {
        double sine = Trig.sin(angle);
        double cosine = Trig.cos(angle);
        return new Affine2D(cosine, sine, -sine, cosine, 0, 0);
    }

    /** Conjugate origin rotation with translations to keep {@code center} fixed. */
    public static Affine2D rotateAround(Point center, double angle) {
        return translate(center.x(), center.y()).multiply(rotate(angle))
                .multiply(translate(-center.x(), -center.y()));
    }

    public static Affine2D scale(double sx, double sy) {
        return new Affine2D(sx, 0, 0, sy, 0, 0);
    }

    public static Affine2D scaleUniform(double factor) {
        return scale(factor, factor);
    }

    public static Affine2D skewX(double angle) {
        return new Affine2D(1, 0, Trig.tan(angle), 1, 0, 0);
    }

    public static Affine2D skewY(double angle) {
        return new Affine2D(1, Trig.tan(angle), 0, 1, 0, 0);
    }

    /** Matrix product: {@code other} acts first, then this matrix. */
    public Affine2D multiply(Affine2D other) {
        return new Affine2D(
                a * other.a + c * other.b,
                b * other.a + d * other.b,
                a * other.c + c * other.d,
                b * other.c + d * other.d,
                a * other.e + c * other.f + e,
                b * other.e + d * other.f + f);
    }

    public Point applyToPoint(Point point) {
        return new Point(a * point.x() + c * point.y() + e,
                b * point.x() + d * point.y() + f);
    }

    public Point applyToVector(Point vector) {
        return new Point(a * vector.x() + c * vector.y(),
                b * vector.x() + d * vector.y());
    }

    public double determinant() {
        return a * d - b * c;
    }

    /** A near-zero area scale has no numerically stable inverse. */
    public Affine2D invert() {
        double det = determinant();
        if (Math.abs(det) < 1e-12) return null;
        double reciprocal = 1.0 / det;
        return new Affine2D(d * reciprocal, -b * reciprocal,
                -c * reciprocal, a * reciprocal,
                (c * f - d * e) * reciprocal,
                (b * e - a * f) * reciprocal);
    }

    public boolean isIdentity() {
        return Math.abs(a - 1) < 1e-10 && Math.abs(b) < 1e-10
                && Math.abs(c) < 1e-10 && Math.abs(d - 1) < 1e-10
                && Math.abs(e) < 1e-10 && Math.abs(f) < 1e-10;
    }

    public boolean isTranslationOnly() {
        return Math.abs(a - 1) < 1e-10 && Math.abs(b) < 1e-10
                && Math.abs(c) < 1e-10 && Math.abs(d - 1) < 1e-10;
    }

    public double[] toArray() {
        return new double[] {a, b, c, d, e, f};
    }
}
