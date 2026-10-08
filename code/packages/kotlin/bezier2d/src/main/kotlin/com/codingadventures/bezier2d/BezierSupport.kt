package com.codingadventures.bezier2d

import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import kotlin.math.abs
import kotlin.math.sqrt

/** Shared polynomial helpers; the G2D02 package has no PHY00 dependency. */
internal object BezierSupport {
    fun validate(tolerance: Double, vararg controls: Point) {
        require(tolerance.isFinite() && tolerance > 0.0) { "tolerance must be finite and positive" }
        controls.forEach(::requireFinite)
    }

    fun requireFinite(point: Point) {
        require(point.x.isFinite() && point.y.isFinite()) { "Bezier point is nonfinite" }
    }

    /** Distance to a *finite* chord, including the zero-length-chord case. */
    fun segmentDistance(control: Point, start: Point, end: Point): Double {
        val chord = end.subtract(start)
        val lengthSquared = chord.magnitudeSquared()
        val u = if (lengthSquared == 0.0) 0.0 else
            (control.subtract(start).dot(chord) / lengthSquared).coerceIn(0.0, 1.0)
        val distance = control.distance(start.add(chord.scale(u)))
        require(distance.isFinite()) { "Bezier flatness overflowed" }
        return distance
    }

    fun bounds(candidates: List<Point>): Rect {
        val minX = candidates.minOf { it.x }
        val minY = candidates.minOf { it.y }
        val maxX = candidates.maxOf { it.x }
        val maxY = candidates.maxOf { it.y }
        return Rect(minX, minY, maxX - minX, maxY - minY)
    }

    /** Interior candidates for a cubic coordinate's quadratic derivative. */
    fun cubicDerivativeRoots(p0: Double, p1: Double, p2: Double, p3: Double): DoubleArray {
        val alpha = p1 - p0
        val beta = 2 * (p0 - 2 * p1 + p2)
        val gamma = -p0 + 3 * p1 - 3 * p2 + p3
        if (abs(gamma) < 1e-12) {
            if (abs(beta) < 1e-12) return doubleArrayOf()
            return doubleArrayOf(-alpha / beta)
        }
        val discriminant = beta * beta - 4 * gamma * alpha
        if (discriminant < 0) return doubleArrayOf()
        val root = sqrt(discriminant)
        return doubleArrayOf((-beta - root) / (2 * gamma), (-beta + root) / (2 * gamma))
    }
}
