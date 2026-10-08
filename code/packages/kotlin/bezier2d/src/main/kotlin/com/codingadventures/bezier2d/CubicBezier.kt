package com.codingadventures.bezier2d

import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import kotlin.math.max

/** Four-point cubic Bezier with exact derivative-root bounds and safe flattening. */
data class CubicBezier(val p0: Point, val p1: Point, val p2: Point, val p3: Point) {
    fun evaluate(t: Double): Point {
        val q0 = p0.lerp(p1, t)
        val q1 = p1.lerp(p2, t)
        val q2 = p2.lerp(p3, t)
        return q0.lerp(q1, t).lerp(q1.lerp(q2, t), t)
    }

    fun derivative(t: Double): Point {
        val complement = 1 - t
        return p1.subtract(p0).scale(complement * complement)
            .add(p2.subtract(p1).scale(2 * complement * t))
            .add(p3.subtract(p2).scale(t * t)).scale(3.0)
    }

    fun split(t: Double): Pair<CubicBezier, CubicBezier> {
        val q0 = p0.lerp(p1, t)
        val q1 = p1.lerp(p2, t)
        val q2 = p2.lerp(p3, t)
        val r0 = q0.lerp(q1, t)
        val r1 = q1.lerp(q2, t)
        val middle = r0.lerp(r1, t)
        return Pair(CubicBezier(p0, q0, r0, middle), CubicBezier(middle, r1, q2, p3))
    }

    fun boundingBox(): Rect {
        val candidates = mutableListOf(p0, p3)
        for (t in BezierSupport.cubicDerivativeRoots(p0.x, p1.x, p2.x, p3.x)) {
            if (t > 0 && t < 1) candidates.add(evaluate(t))
        }
        for (t in BezierSupport.cubicDerivativeRoots(p0.y, p1.y, p2.y, p3.y)) {
            if (t > 0 && t < 1) candidates.add(evaluate(t))
        }
        return BezierSupport.bounds(candidates)
    }

    fun toPolyline(tolerance: Double): List<Point> {
        BezierSupport.validate(tolerance, p0, p1, p2, p3)
        val points = mutableListOf(p0)
        visit(this, tolerance, 0, intArrayOf(0), points)
        return points.toList()
    }

    private fun visit(curve: CubicBezier, tolerance: Double, depth: Int,
                      subdivisions: IntArray, points: MutableList<Point>) {
        // The convex hull's maximum control-to-finite-chord distance is a
        // conservative bound. A midpoint sample is not (symmetric S-curve).
        val error = max(
            BezierSupport.segmentDistance(curve.p1, curve.p0, curve.p3),
            BezierSupport.segmentDistance(curve.p2, curve.p0, curve.p3),
        )
        if (error <= tolerance) {
            points.add(curve.p3)
            return
        }
        if (depth == 32 || subdivisions[0] == 65535) {
            throw IllegalStateException("Bezier subdivision budget exhausted")
        }
        subdivisions[0]++
        val (left, right) = curve.split(0.5)
        BezierSupport.requireFinite(left.p1)
        BezierSupport.requireFinite(left.p2)
        BezierSupport.requireFinite(left.p3)
        BezierSupport.requireFinite(right.p1)
        BezierSupport.requireFinite(right.p2)
        visit(left, tolerance, depth + 1, subdivisions, points)
        visit(right, tolerance, depth + 1, subdivisions, points)
    }
}
