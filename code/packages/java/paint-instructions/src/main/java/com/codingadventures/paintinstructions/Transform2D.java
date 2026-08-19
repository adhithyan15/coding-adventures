package com.codingadventures.paintinstructions;

import java.util.Objects;

/**
 * A six-value affine transform, matching the Canvas/SVG convention:
 *
 * <pre>
 *   x' = a*x + c*y + e
 *   y' = b*x + d*y + f
 * </pre>
 */
public final class Transform2D {
    public final double a;
    public final double b;
    public final double c;
    public final double d;
    public final double e;
    public final double f;

    /** The identity transform — no rotation, scale, or translation. */
    public static final Transform2D IDENTITY = new Transform2D(1, 0, 0, 1, 0, 0);

    public Transform2D(double a, double b, double c, double d, double e, double f) {
        this.a = a;
        this.b = b;
        this.c = c;
        this.d = d;
        this.e = e;
        this.f = f;
    }

    /** Whether this transform is (bitwise) equal to the identity transform. */
    public boolean isIdentity() {
        return a == 1 && b == 0 && c == 0 && d == 1 && e == 0 && f == 0;
    }

    @Override
    public String toString() {
        return "Transform2D{a=" + a + ", b=" + b + ", c=" + c + ", d=" + d + ", e=" + e + ", f=" + f + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof Transform2D other)) return false;
        return Double.compare(a, other.a) == 0 && Double.compare(b, other.b) == 0 &&
                Double.compare(c, other.c) == 0 && Double.compare(d, other.d) == 0 &&
                Double.compare(e, other.e) == 0 && Double.compare(f, other.f) == 0;
    }

    @Override
    public int hashCode() {
        return Objects.hash(a, b, c, d, e, f);
    }
}
