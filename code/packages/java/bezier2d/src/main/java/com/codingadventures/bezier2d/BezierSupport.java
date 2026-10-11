package com.codingadventures.bezier2d;

import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import java.util.List;

/** Shared finite-chord geometry; neither curve needs trigonometry. */
final class BezierSupport {
    private BezierSupport() {}

    static void validate(double tolerance, Point... controls) {
        if (!Double.isFinite(tolerance) || tolerance <= 0) {
            throw new IllegalArgumentException("tolerance must be finite and positive");
        }
        for (Point point : controls) requireFinite(point);
    }

    static void requireFinite(Point point) {
        if (!Double.isFinite(point.x()) || !Double.isFinite(point.y())) {
            throw new IllegalArgumentException("Bezier control/intermediate point is nonfinite");
        }
    }

    /** Project onto the finite segment, not its infinite supporting line. */
    static double segmentDistance(Point control, Point start, Point end) {
        Point chord = end.subtract(start);
        double lengthSquared = chord.magnitudeSquared();
        double u = lengthSquared == 0 ? 0 : Math.max(0, Math.min(1,
                control.subtract(start).dot(chord) / lengthSquared));
        double distance = control.distance(start.add(chord.scale(u)));
        if (!Double.isFinite(distance)) {
            throw new IllegalArgumentException("Bezier flatness overflowed");
        }
        return distance;
    }

    static Rect bounds(List<Point> candidates) {
        double minX = Double.POSITIVE_INFINITY;
        double minY = Double.POSITIVE_INFINITY;
        double maxX = Double.NEGATIVE_INFINITY;
        double maxY = Double.NEGATIVE_INFINITY;
        for (Point point : candidates) {
            minX = Math.min(minX, point.x());
            minY = Math.min(minY, point.y());
            maxX = Math.max(maxX, point.x());
            maxY = Math.max(maxY, point.y());
        }
        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    /** Roots of one cubic coordinate's derivative inside the open unit interval. */
    static double[] cubicDerivativeRoots(double p0, double p1, double p2, double p3) {
        double alpha = p1 - p0;
        double beta = 2 * (p0 - 2 * p1 + p2);
        double gamma = -p0 + 3 * p1 - 3 * p2 + p3;
        if (Math.abs(gamma) < 1e-12) {
            if (Math.abs(beta) < 1e-12) return new double[0];
            return new double[] {-alpha / beta};
        }
        double discriminant = beta * beta - 4 * gamma * alpha;
        if (discriminant < 0) return new double[0];
        double root = Math.sqrt(discriminant);
        return new double[] {(-beta - root) / (2 * gamma), (-beta + root) / (2 * gamma)};
    }
}
