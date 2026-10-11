package com.codingadventures.bezier2d

import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import kotlin.math.abs

/** Three-point quadratic Bezier with de Casteljau evaluation and subdivision. */
data class QuadraticBezier(val p0: Point, val p1: Point, val p2: Point) {
    fun evaluate(t: Double): Point = p0.lerp(p1, t).lerp(p1.lerp(p2, t), t)

    fun derivative(t: Double): Point = p1.subtract(p0).scale(1 - t)
        .add(p2.subtract(p1).scale(t)).scale(2.0)

    fun split(t: Double): Pair<QuadraticBezier, QuadraticBezier> {
        val q0 = p0.lerp(p1, t)
        val q1 = p1.lerp(p2, t)
        val middle = q0.lerp(q1, t)
        return Pair(QuadraticBezier(p0, q0, middle), QuadraticBezier(middle, q1, p2))
    }

    fun elevate(): CubicBezier = CubicBezier(p0,
        p0.scale(1.0 / 3).add(p1.scale(2.0 / 3)),
        p1.scale(2.0 / 3).add(p2.scale(1.0 / 3)), p2)

    fun boundingBox(): Rect {
        val candidates = mutableListOf(p0, p2)
        val xDenominator = p0.x - 2 * p1.x + p2.x
        val yDenominator = p0.y - 2 * p1.y + p2.y
        if (abs(xDenominator) >= 1e-12) {
            val t = (p0.x - p1.x) / xDenominator
            if (t > 0 && t < 1) candidates.add(evaluate(t))
        }
        if (abs(yDenominator) >= 1e-12) {
            val t = (p0.y - p1.y) / yDenominator
            if (t > 0 && t < 1) candidates.add(evaluate(t))
        }
        return BezierSupport.bounds(candidates)
    }

    fun toPolyline(tolerance: Double): List<Point> {
        BezierSupport.validate(tolerance, p0, p1, p2)
        val points = mutableListOf(p0)
        visit(this, tolerance, 0, intArrayOf(0), points)
        return points.toList()
    }

    private fun visit(curve: QuadraticBezier, tolerance: Double, depth: Int,
                      subdivisions: IntArray, points: MutableList<Point>) {
        val error = BezierSupport.segmentDistance(curve.p1, curve.p0, curve.p2)
        if (error <= tolerance) {
            points.add(curve.p2)
            return
        }
        if (depth == 32 || subdivisions[0] == 65535) {
            throw IllegalStateException("Bezier subdivision budget exhausted")
        }
        subdivisions[0]++
        val (left, right) = curve.split(0.5)
        BezierSupport.requireFinite(left.p1)
        BezierSupport.requireFinite(left.p2)
        BezierSupport.requireFinite(right.p1)
        visit(left, tolerance, depth + 1, subdivisions, points)
        visit(right, tolerance, depth + 1, subdivisions, points)
    }
}
