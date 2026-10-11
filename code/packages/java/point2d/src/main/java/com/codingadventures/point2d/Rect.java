package com.codingadventures.point2d;

/**
 * An immutable axis-aligned bounding box with a half-open right/bottom edge.
 *
 * <p>The interval [x, x + width) by [y, y + height) lets neighboring boxes
 * tile the plane without counting a shared boundary point twice.
 */
public record Rect(double x, double y, double width, double height) {
    public static Rect fromPoints(Point min, Point max) {
        return new Rect(min.x(), min.y(), max.x() - min.x(), max.y() - min.y());
    }

    public static Rect zero() {
        return new Rect(0.0, 0.0, 0.0, 0.0);
    }

    public Point min() {
        return new Point(x, y);
    }

    public Point max() {
        return new Point(x + width, y + height);
    }

    public Point center() {
        return new Point(x + width / 2.0, y + height / 2.0);
    }

    public boolean isEmpty() {
        return width <= 0.0 || height <= 0.0;
    }

    public boolean containsPoint(Point point) {
        return x <= point.x() && point.x() < x + width
                && y <= point.y() && point.y() < y + height;
    }

    /** Empty boxes are identities, not extra points at their origins. */
    public Rect union(Rect other) {
        if (isEmpty()) return other;
        if (other.isEmpty()) return this;
        double left = Math.min(x, other.x);
        double top = Math.min(y, other.y);
        double right = Math.max(x + width, other.x + other.width);
        double bottom = Math.max(y + height, other.y + other.height);
        return new Rect(left, top, right - left, bottom - top);
    }

    /** A zero-width or zero-height touch is not an area intersection. */
    public Rect intersection(Rect other) {
        double left = Math.max(x, other.x);
        double top = Math.max(y, other.y);
        double overlapWidth = Math.min(x + width, other.x + other.width) - left;
        double overlapHeight = Math.min(y + height, other.y + other.height) - top;
        if (overlapWidth <= 0.0 || overlapHeight <= 0.0) return null;
        return new Rect(left, top, overlapWidth, overlapHeight);
    }

    /** Move each edge outward by the same amount. */
    public Rect expandBy(double amount) {
        return new Rect(x - amount, y - amount, width + 2 * amount, height + 2 * amount);
    }
}
