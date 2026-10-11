package com.codingadventures.point2d

import com.codingadventures.trig.Trig
import kotlin.math.sqrt

/**
 * An immutable 2D position or displacement (G2D00).
 *
 * A pair of Cartesian coordinates can mean "where" relative to the origin
 * or "how far" relative to another point. Every operation returns a new
 * value, leaving the two operands safe to reuse.
 */
data class Point(val x: Double, val y: Double) {
    companion object {
        /** Additive identity, also the answer for an unstable direction. */
        fun origin(): Point = Point(0.0, 0.0)
    }

    fun add(other: Point): Point = Point(x + other.x, y + other.y)

    /** Displacement from [other] to this point. */
    fun subtract(other: Point): Point = Point(x - other.x, y - other.y)

    fun scale(scalar: Double): Point = Point(x * scalar, y * scalar)
    fun negate(): Point = Point(-x, -y)

    /** Zero means the two vectors are perpendicular. */
    fun dot(other: Point): Double = x * other.x + y * other.y

    /** Positive means [other] turns counterclockwise from this vector. */
    fun cross(other: Point): Double = x * other.y - y * other.x

    fun magnitudeSquared(): Double = x * x + y * y
    fun magnitude(): Double = sqrt(magnitudeSquared())

    /** Compare the length itself, keeping exactly 1e-12 normalizable. */
    fun normalize(): Point {
        val length = magnitude()
        return if (length < 1e-12) origin() else Point(x / length, y / length)
    }

    fun distanceSquared(other: Point): Double = subtract(other).magnitudeSquared()
    fun distance(other: Point): Double = subtract(other).magnitude()

    /** A + t(B - A), also defined for extrapolation outside [0, 1]. */
    fun lerp(other: Point, t: Double): Point = add(other.subtract(this).scale(t))

    /** Quarter-turn counterclockwise without trigonometry. */
    fun perpendicular(): Point = Point(-y, x)

    /** The only trigonometric operation flows through the local PHY00 root. */
    fun angle(): Double = Trig.atan2(y, x)
}
