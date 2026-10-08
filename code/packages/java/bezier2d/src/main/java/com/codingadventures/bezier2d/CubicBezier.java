package com.codingadventures.bezier2d;

import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import java.util.ArrayList;
import java.util.List;

/** Four control points define a cubic Bezier segment (G2D02). */
public record CubicBezier(Point p0, Point p1, Point p2, Point p3) {
    public Point evaluate(double t) {
        Point q0 = p0.lerp(p1, t);
        Point q1 = p1.lerp(p2, t);
        Point q2 = p2.lerp(p3, t);
        return q0.lerp(q1, t).lerp(q1.lerp(q2, t), t);
    }

    public Point derivative(double t) {
        double complement = 1 - t;
        return p1.subtract(p0).scale(complement * complement)
                .add(p2.subtract(p1).scale(2 * complement * t))
                .add(p3.subtract(p2).scale(t * t)).scale(3);
    }

    public Split<CubicBezier> split(double t) {
        Point q0 = p0.lerp(p1, t);
        Point q1 = p1.lerp(p2, t);
        Point q2 = p2.lerp(p3, t);
        Point r0 = q0.lerp(q1, t);
        Point r1 = q1.lerp(q2, t);
        Point middle = r0.lerp(r1, t);
        return new Split<>(new CubicBezier(p0, q0, r0, middle),
                new CubicBezier(middle, r1, q2, p3));
    }

    public Rect boundingBox() {
        List<Point> candidates = new ArrayList<>(List.of(p0, p3));
        for (double t : BezierSupport.cubicDerivativeRoots(p0.x(), p1.x(), p2.x(), p3.x())) {
            if (t > 0 && t < 1) candidates.add(evaluate(t));
        }
        for (double t : BezierSupport.cubicDerivativeRoots(p0.y(), p1.y(), p2.y(), p3.y())) {
            if (t > 0 && t < 1) candidates.add(evaluate(t));
        }
        return BezierSupport.bounds(candidates);
    }

    public List<Point> toPolyline(double tolerance) {
        BezierSupport.validate(tolerance, p0, p1, p2, p3);
        List<Point> points = new ArrayList<>();
        points.add(p0);
        visit(this, tolerance, 0, new int[1], points);
        return List.copyOf(points);
    }

    private static void visit(CubicBezier curve, double tolerance, int depth,
                              int[] subdivisions, List<Point> points) {
        // A midpoint sample can lie on the chord while the curve bows away.
        // The maximum control-to-finite-segment distance bounds the whole hull.
        double error = Math.max(
                BezierSupport.segmentDistance(curve.p1, curve.p0, curve.p3),
                BezierSupport.segmentDistance(curve.p2, curve.p0, curve.p3));
        if (error <= tolerance) {
            points.add(curve.p3);
            return;
        }
        if (depth == 32 || subdivisions[0] == 65535) {
            throw new IllegalStateException("Bezier subdivision budget exhausted");
        }
        subdivisions[0]++;
        Split<CubicBezier> halves = curve.split(0.5);
        BezierSupport.requireFinite(halves.left().p1);
        BezierSupport.requireFinite(halves.left().p2);
        BezierSupport.requireFinite(halves.left().p3);
        BezierSupport.requireFinite(halves.right().p1);
        BezierSupport.requireFinite(halves.right().p2);
        visit(halves.left(), tolerance, depth + 1, subdivisions, points);
        visit(halves.right(), tolerance, depth + 1, subdivisions, points);
    }
}
