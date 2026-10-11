package com.codingadventures.arc2d;

import com.codingadventures.bezier2d.CubicBezier;
import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import com.codingadventures.trig.Trig;
import java.util.List;

/** SVG endpoint-form elliptical arc with W3C conversion and line fallback. */
public record SvgArc(Point from, Point to, double rx, double ry, double xRotation,
                     boolean largeArc, boolean sweep) {
    private static void finite(double value) {
        if (!Double.isFinite(value)) throw new IllegalArgumentException("nonfinite SVG arc geometry");
    }

    /** Returns null for a degenerate line/point arc. */
    public CenterArc toCenterArc() {
        if (from == null || to == null) throw new IllegalArgumentException("missing endpoint");
        finite(from.x()); finite(from.y()); finite(to.x()); finite(to.y());
        finite(rx); finite(ry); finite(xRotation);
        double radiusX = Math.abs(rx), radiusY = Math.abs(ry);
        if (radiusX < 1e-10 || radiusY < 1e-10 || from.distanceSquared(to) < 1e-20) return null;
        double cosR = Trig.cos(xRotation), sinR = Trig.sin(xRotation);
        double dx = (from.x() - to.x()) / 2, dy = (from.y() - to.y()) / 2;
        double x1 = cosR * dx + sinR * dy, y1 = -sinR * dx + cosR * dy;
        finite(x1); finite(y1);
        double xRatio = x1 / radiusX, yRatio = y1 / radiusY;
        double lambda = xRatio * xRatio + yRatio * yRatio;
        finite(lambda);
        if (lambda > 1) {
            double scale = Trig.sqrt(lambda);
            radiusX *= scale; radiusY *= scale;
            xRatio = x1 / radiusX; yRatio = y1 / radiusY;
            lambda = xRatio * xRatio + yRatio * yRatio;
        }
        finite(radiusX); finite(radiusY);
        if (lambda <= 0) throw new IllegalArgumentException("unstable SVG center");
        double sign = largeArc != sweep ? 1 : -1;
        double factor = sign * Trig.sqrt(Math.max(0, (1 - lambda) / lambda));
        double cxLocal = factor * radiusX * yRatio;
        double cyLocal = -factor * radiusY * xRatio;
        double cx = cosR * cxLocal - sinR * cyLocal + (from.x() + to.x()) / 2;
        double cy = sinR * cxLocal + cosR * cyLocal + (from.y() + to.y()) / 2;
        finite(cx); finite(cy);
        double start = Trig.atan2((y1 - cyLocal) / radiusY, (x1 - cxLocal) / radiusX);
        double end = Trig.atan2((-y1 - cyLocal) / radiusY, (-x1 - cxLocal) / radiusX);
        double delta = end - start;
        if (!sweep && delta > 0) delta -= 2 * Trig.PI;
        if (sweep && delta < 0) delta += 2 * Trig.PI;
        delta = Math.max(-2 * Trig.PI, Math.min(2 * Trig.PI, delta));
        finite(start); finite(delta);
        return new CenterArc(new Point(cx, cy), radiusX, radiusY, start, delta, xRotation);
    }

    public Point evaluate(double t) {
        finite(t);
        CenterArc arc = toCenterArc();
        if (arc != null) return arc.evaluate(t);
        Point point = from.lerp(to, t);
        finite(point.x()); finite(point.y());
        return point;
    }

    public Rect boundingBox() {
        CenterArc arc = toCenterArc();
        if (arc != null) return arc.boundingBox();
        Rect bounds = Rect.fromPoints(new Point(Math.min(from.x(), to.x()), Math.min(from.y(), to.y())),
                new Point(Math.max(from.x(), to.x()), Math.max(from.y(), to.y())));
        finite(bounds.width()); finite(bounds.height());
        return bounds;
    }

    public List<CubicBezier> toCubicBeziers() {
        CenterArc arc = toCenterArc();
        return arc == null ? List.of() : arc.toCubicBeziers();
    }
}
