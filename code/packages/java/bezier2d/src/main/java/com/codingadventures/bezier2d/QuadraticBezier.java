package com.codingadventures.bezier2d;

import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import java.util.ArrayList;
import java.util.List;

/** Three control points define a quadratic Bezier segment (G2D02). */
public record QuadraticBezier(Point p0, Point p1, Point p2) {
    /** Repeated interpolation is more stable than expanded polynomial powers. */
    public Point evaluate(double t) {
        return p0.lerp(p1, t).lerp(p1.lerp(p2, t), t);
    }

    public Point derivative(double t) {
        return p1.subtract(p0).scale(1 - t)
                .add(p2.subtract(p1).scale(t)).scale(2);
    }

    public Split<QuadraticBezier> split(double t) {
        Point q0 = p0.lerp(p1, t);
        Point q1 = p1.lerp(p2, t);
        Point middle = q0.lerp(q1, t);
        return new Split<>(new QuadraticBezier(p0, q0, middle),
                new QuadraticBezier(middle, q1, p2));
    }

    public CubicBezier elevate() {
        return new CubicBezier(p0, p0.scale(1.0 / 3).add(p1.scale(2.0 / 3)),
                p1.scale(2.0 / 3).add(p2.scale(1.0 / 3)), p2);
    }

    public Rect boundingBox() {
        List<Point> candidates = new ArrayList<>(List.of(p0, p2));
        double txDenominator = p0.x() - 2 * p1.x() + p2.x();
        double tyDenominator = p0.y() - 2 * p1.y() + p2.y();
        if (Math.abs(txDenominator) >= 1e-12) {
            double t = (p0.x() - p1.x()) / txDenominator;
            if (t > 0 && t < 1) candidates.add(evaluate(t));
        }
        if (Math.abs(tyDenominator) >= 1e-12) {
            double t = (p0.y() - p1.y()) / tyDenominator;
            if (t > 0 && t < 1) candidates.add(evaluate(t));
        }
        return BezierSupport.bounds(candidates);
    }

    public List<Point> toPolyline(double tolerance) {
        BezierSupport.validate(tolerance, p0, p1, p2);
        List<Point> points = new ArrayList<>();
        points.add(p0);
        visit(this, tolerance, 0, new int[1], points);
        return List.copyOf(points);
    }

    /** A failed recursive visit throws, so no partly accumulated line escapes. */
    private static void visit(QuadraticBezier curve, double tolerance, int depth,
                              int[] subdivisions, List<Point> points) {
        double error = BezierSupport.segmentDistance(curve.p1, curve.p0, curve.p2);
        if (error <= tolerance) {
            points.add(curve.p2);
            return;
        }
        if (depth == 32 || subdivisions[0] == 65535) {
            throw new IllegalStateException("Bezier subdivision budget exhausted");
        }
        subdivisions[0]++;
        Split<QuadraticBezier> halves = curve.split(0.5);
        BezierSupport.requireFinite(halves.left().p1);
        BezierSupport.requireFinite(halves.left().p2);
        BezierSupport.requireFinite(halves.right().p1);
        visit(halves.left(), tolerance, depth + 1, subdivisions, points);
        visit(halves.right(), tolerance, depth + 1, subdivisions, points);
    }
}
