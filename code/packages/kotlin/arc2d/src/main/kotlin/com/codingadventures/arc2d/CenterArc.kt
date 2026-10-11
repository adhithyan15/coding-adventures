package com.codingadventures.arc2d

import com.codingadventures.bezier2d.CubicBezier
import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import com.codingadventures.trig.Trig
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min

/** Bounded elliptical arc in center/angle form (G2D03). */
data class CenterArc(val center: Point, val rx: Double, val ry: Double, val startAngle: Double,
                     val sweepAngle: Double, val xRotation: Double) {
    private fun finite(value: Double) {
        require(value.isFinite()) { "nonfinite arc geometry" }
    }

    private fun checked(point: Point): Point {
        finite(point.x); finite(point.y)
        return point
    }

    private fun validate() {
        checked(center)
        finite(rx); finite(ry); finite(startAngle); finite(sweepAngle); finite(xRotation)
        require(rx > 0 && ry > 0) { "radii must be positive" }
        require(abs(sweepAngle) <= 2 * Trig.PI) { "invalid-sweep" }
        finite(startAngle + sweepAngle)
    }

    private fun at(angle: Double): Point {
        val cosR = Trig.cos(xRotation)
        val sinR = Trig.sin(xRotation)
        val xp = rx * Trig.cos(angle)
        val yp = ry * Trig.sin(angle)
        return checked(Point(cosR * xp - sinR * yp + center.x, sinR * xp + cosR * yp + center.y))
    }

    private fun derivativeAt(angle: Double): Point {
        val cosR = Trig.cos(xRotation)
        val sinR = Trig.sin(xRotation)
        val dx = -rx * Trig.sin(angle)
        val dy = ry * Trig.cos(angle)
        return checked(Point(cosR * dx - sinR * dy, sinR * dx + cosR * dy))
    }

    fun evaluate(t: Double): Point {
        validate(); finite(t)
        val angle = startAngle + t * sweepAngle
        finite(angle)
        return at(angle)
    }

    fun tangent(t: Double): Point {
        validate(); finite(t)
        val angle = startAngle + t * sweepAngle
        finite(angle)
        return checked(derivativeAt(angle).scale(sweepAngle))
    }

    private fun positiveMod(value: Double): Double {
        val residue = value % (2 * Trig.PI)
        return if (residue < 0) residue + 2 * Trig.PI else residue
    }

    private fun includes(angle: Double): Boolean = when {
        sweepAngle > 0 -> positiveMod(angle - startAngle) <= sweepAngle
        sweepAngle < 0 -> positiveMod(startAngle - angle) <= -sweepAngle
        else -> false
    }

    /** Exact analytic extrema with directed periodic membership. */
    fun boundingBox(): Rect {
        validate()
        val points = mutableListOf(at(startAngle), at(startAngle + sweepAngle))
        val cosR = Trig.cos(xRotation)
        val sinR = Trig.sin(xRotation)
        val x = Trig.atan2(-ry * sinR, rx * cosR)
        val y = Trig.atan2(ry * cosR, rx * sinR)
        for (candidate in listOf(x, x + Trig.PI, y, y + Trig.PI)) {
            if (includes(candidate)) points.add(at(candidate))
        }
        var minX = Double.POSITIVE_INFINITY
        var minY = Double.POSITIVE_INFINITY
        var maxX = Double.NEGATIVE_INFINITY
        var maxY = Double.NEGATIVE_INFINITY
        for (point in points) {
            minX = min(minX, point.x); minY = min(minY, point.y)
            maxX = max(maxX, point.x); maxY = max(maxY, point.y)
        }
        finite(maxX - minX); finite(maxY - minY)
        return Rect(minX, minY, maxX - minX, maxY - minY)
    }

    /** One to four cubic segments, each no more than a quarter turn. */
    fun toCubicBeziers(): List<CubicBezier> {
        validate()
        val count = max(1, ceil(abs(sweepAngle) / (Trig.PI / 2)).toInt())
        require(count <= 4) { "invalid-sweep" }
        val segmentSweep = sweepAngle / count
        val k = 4.0 / 3.0 * Trig.tan(segmentSweep / 4.0)
        finite(k)
        return (0 until count).map { index ->
            val a = startAngle + index * segmentSweep
            val b = a + segmentSweep
            val p0 = at(a)
            val p3 = at(b)
            val p1 = checked(p0.add(derivativeAt(a).scale(k)))
            val p2 = checked(p3.subtract(derivativeAt(b).scale(k)))
            CubicBezier(p0, p1, p2, p3)
        }
    }
}
