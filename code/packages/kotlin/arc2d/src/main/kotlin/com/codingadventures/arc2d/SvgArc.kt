package com.codingadventures.arc2d

import com.codingadventures.bezier2d.CubicBezier
import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import com.codingadventures.trig.Trig
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

/** SVG endpoint-form elliptical arc with W3C conversion and line fallback. */
data class SvgArc(val from: Point, val to: Point, val rx: Double, val ry: Double,
                  val xRotation: Double, val largeArc: Boolean, val sweep: Boolean) {
    private fun finite(value: Double) {
        require(value.isFinite()) { "nonfinite SVG arc geometry" }
    }

    fun toCenterArc(): CenterArc? {
        finite(from.x); finite(from.y); finite(to.x); finite(to.y)
        finite(rx); finite(ry); finite(xRotation)
        var radiusX = abs(rx)
        var radiusY = abs(ry)
        if (radiusX < 1e-10 || radiusY < 1e-10 || from.distanceSquared(to) < 1e-20) return null
        val cosR = Trig.cos(xRotation)
        val sinR = Trig.sin(xRotation)
        val dx = (from.x - to.x) / 2
        val dy = (from.y - to.y) / 2
        val x1 = cosR * dx + sinR * dy
        val y1 = -sinR * dx + cosR * dy
        finite(x1); finite(y1)
        var xRatio = x1 / radiusX
        var yRatio = y1 / radiusY
        var lambda = xRatio * xRatio + yRatio * yRatio
        finite(lambda)
        if (lambda > 1) {
            val scale = Trig.sqrt(lambda)
            radiusX *= scale; radiusY *= scale
            xRatio = x1 / radiusX; yRatio = y1 / radiusY
            lambda = xRatio * xRatio + yRatio * yRatio
        }
        finite(radiusX); finite(radiusY)
        require(lambda > 0) { "unstable SVG center" }
        val sign = if (largeArc != sweep) 1.0 else -1.0
        val factor = sign * Trig.sqrt(max(0.0, (1 - lambda) / lambda))
        val cxLocal = factor * radiusX * yRatio
        val cyLocal = -factor * radiusY * xRatio
        val cx = cosR * cxLocal - sinR * cyLocal + (from.x + to.x) / 2
        val cy = sinR * cxLocal + cosR * cyLocal + (from.y + to.y) / 2
        finite(cx); finite(cy)
        val start = Trig.atan2((y1 - cyLocal) / radiusY, (x1 - cxLocal) / radiusX)
        val end = Trig.atan2((-y1 - cyLocal) / radiusY, (-x1 - cxLocal) / radiusX)
        var delta = end - start
        if (!sweep && delta > 0) delta -= 2 * Trig.PI
        if (sweep && delta < 0) delta += 2 * Trig.PI
        delta = max(-2 * Trig.PI, min(2 * Trig.PI, delta))
        finite(start); finite(delta)
        return CenterArc(Point(cx, cy), radiusX, radiusY, start, delta, xRotation)
    }

    fun evaluate(t: Double): Point {
        finite(t)
        val arc = toCenterArc()
        if (arc != null) return arc.evaluate(t)
        val point = from.lerp(to, t)
        finite(point.x); finite(point.y)
        return point
    }

    fun boundingBox(): Rect {
        val arc = toCenterArc()
        if (arc != null) return arc.boundingBox()
        val bounds = Rect.fromPoints(Point(min(from.x, to.x), min(from.y, to.y)),
            Point(max(from.x, to.x), max(from.y, to.y)))
        finite(bounds.width); finite(bounds.height)
        return bounds
    }

    fun toCubicBeziers(): List<CubicBezier> = toCenterArc()?.toCubicBeziers() ?: emptyList()
}
