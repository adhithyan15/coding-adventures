package com.codingadventures.arc2d;

import com.codingadventures.bezier2d.CubicBezier;
import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import com.codingadventures.trig.Trig;
import java.util.ArrayList;
import java.util.List;

/** A bounded elliptical arc in center/angle form (G2D03). */
public record CenterArc(Point center, double rx, double ry, double startAngle,
                        double sweepAngle, double xRotation) {
    private static final double TWO_PI = 2 * Trig.PI;

    private static void finite(double value) {
        if (!Double.isFinite(value)) throw new IllegalArgumentException("nonfinite arc geometry");
    }

    private static Point checked(Point point) {
        finite(point.x());
        finite(point.y());
        return point;
    }

    private void validate() {
        if (center == null) throw new IllegalArgumentException("missing arc center");
        checked(center);
        finite(rx); finite(ry); finite(startAngle); finite(sweepAngle); finite(xRotation);
        if (rx <= 0 || ry <= 0) throw new IllegalArgumentException("radii must be positive");
        if (Math.abs(sweepAngle) > TWO_PI) throw new IllegalArgumentException("invalid-sweep");
        // A finite input can overflow during angle or radius arithmetic.
        finite(startAngle + sweepAngle);
    }

    private Point at(double angle) {
        double cosR = Trig.cos(xRotation), sinR = Trig.sin(xRotation);
        double xp = rx * Trig.cos(angle), yp = ry * Trig.sin(angle);
        return checked(new Point(cosR * xp - sinR * yp + center.x(),
                sinR * xp + cosR * yp + center.y()));
    }

    private Point derivativeAt(double angle) {
        double cosR = Trig.cos(xRotation), sinR = Trig.sin(xRotation);
        double dx = -rx * Trig.sin(angle), dy = ry * Trig.cos(angle);
        return checked(new Point(cosR * dx - sinR * dy, sinR * dx + cosR * dy));
    }

    /** Point at normalized parameter {@code t}; extrapolation is allowed. */
    public Point evaluate(double t) {
        validate();
        finite(t);
        double angle = startAngle + t * sweepAngle;
        finite(angle);
        return at(angle);
    }

    /** Derivative with respect to normalized parameter {@code t}. */
    public Point tangent(double t) {
        validate();
        finite(t);
        double angle = startAngle + t * sweepAngle;
        finite(angle);
        return checked(derivativeAt(angle).scale(sweepAngle));
    }

    private static double positiveMod(double value) {
        double residue = value % TWO_PI;
        return residue < 0 ? residue + TWO_PI : residue;
    }

    private boolean includes(double angle) {
        if (sweepAngle > 0) return positiveMod(angle - startAngle) <= sweepAngle;
        if (sweepAngle < 0) return positiveMod(startAngle - angle) <= -sweepAngle;
        return false;
    }

    /** Exact analytic extrema, including directed crossings of the 2π seam. */
    public Rect boundingBox() {
        validate();
        List<Point> points = new ArrayList<>(List.of(at(startAngle), at(startAngle + sweepAngle)));
        double cosR = Trig.cos(xRotation), sinR = Trig.sin(xRotation);
        double x = Trig.atan2(-ry * sinR, rx * cosR);
        double y = Trig.atan2(ry * cosR, rx * sinR);
        for (double candidate : new double[] {x, x + Trig.PI, y, y + Trig.PI}) {
            if (includes(candidate)) points.add(at(candidate));
        }
        double minX = Double.POSITIVE_INFINITY, minY = Double.POSITIVE_INFINITY;
        double maxX = Double.NEGATIVE_INFINITY, maxY = Double.NEGATIVE_INFINITY;
        for (Point point : points) {
            minX = Math.min(minX, point.x()); minY = Math.min(minY, point.y());
            maxX = Math.max(maxX, point.x()); maxY = Math.max(maxY, point.y());
        }
        finite(maxX - minX); finite(maxY - minY);
        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    /** At most four cubic segments, each spanning at most a quarter turn. */
    public List<CubicBezier> toCubicBeziers() {
        validate();
        int count = Math.max(1, (int) Math.ceil(Math.abs(sweepAngle) / (Trig.PI / 2)));
        if (count > 4) throw new IllegalArgumentException("invalid-sweep");
        double segmentSweep = sweepAngle / count;
        double k = (4.0 / 3.0) * Trig.tan(segmentSweep / 4.0);
        finite(k);
        List<CubicBezier> curves = new ArrayList<>(count);
        for (int index = 0; index < count; index++) {
            double a = startAngle + index * segmentSweep;
            double b = a + segmentSweep;
            Point p0 = at(a), p3 = at(b);
            Point p1 = checked(p0.add(derivativeAt(a).scale(k)));
            Point p2 = checked(p3.subtract(derivativeAt(b).scale(k)));
            curves.add(new CubicBezier(p0, p1, p2, p3));
        }
        return List.copyOf(curves);
    }
}
