package com.codingadventures.point2d;

import com.codingadventures.trig.Trig;

/**
 * An immutable position or displacement in the Cartesian plane (G2D00).
 *
 * <p>The same pair of coordinates describes a position from the origin or a
 * vector between positions. Every operation returns a new value, so callers
 * can reuse the operands after translation, interpolation, or normalization.
 */
public record Point(double x, double y) {
    /** The additive identity and conventional answer for directionless vectors. */
    public static Point origin() {
        return new Point(0.0, 0.0);
    }

    /** Translate this point by {@code other}, component by component. */
    public Point add(Point other) {
        return new Point(x + other.x, y + other.y);
    }

    /** The displacement from {@code other} to this point. */
    public Point subtract(Point other) {
        return new Point(x - other.x, y - other.y);
    }

    /** Stretch, shrink, or reverse a vector without changing the original. */
    public Point scale(double scalar) {
        return new Point(x * scalar, y * scalar);
    }

    public Point negate() {
        return new Point(-x, -y);
    }

    /** Projection numerator: zero means perpendicular vectors. */
    public double dot(Point other) {
        return x * other.x + y * other.y;
    }

    /** Signed 2D orientation: positive turns counterclockwise. */
    public double cross(Point other) {
        return x * other.y - y * other.x;
    }

    public double magnitudeSquared() {
        return x * x + y * y;
    }

    public double magnitude() {
        return Math.sqrt(magnitudeSquared());
    }

    /**
     * Divide by length only when a stable direction exists. The comparison
     * is on length, not squared length: exactly 1e-12 remains normalizable.
     */
    public Point normalize() {
        double length = magnitude();
        if (length < 1e-12) {
            return origin();
        }
        return new Point(x / length, y / length);
    }

    public double distanceSquared(Point other) {
        return subtract(other).magnitudeSquared();
    }

    public double distance(Point other) {
        return subtract(other).magnitude();
    }

    /** A + t(B - A), including extrapolation when t is outside [0, 1]. */
    public Point lerp(Point other, double t) {
        return add(other.subtract(this).scale(t));
    }

    /** Rotate a displacement one quarter-turn counterclockwise. */
    public Point perpendicular() {
        return new Point(-y, x);
    }

    /** Route the quadrant-sensitive angle through the local PHY00 package. */
    public double angle() {
        return Trig.atan2(y, x);
    }
}
