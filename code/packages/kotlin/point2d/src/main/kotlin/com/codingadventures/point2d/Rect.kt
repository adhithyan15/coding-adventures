package com.codingadventures.point2d

import kotlin.math.max
import kotlin.math.min

/** Half-open axis-aligned box: its right and bottom edges are excluded. */
data class Rect(val x: Double, val y: Double, val width: Double, val height: Double) {
    companion object {
        fun fromPoints(min: Point, max: Point): Rect =
            Rect(min.x, min.y, max.x - min.x, max.y - min.y)

        fun zero(): Rect = Rect(0.0, 0.0, 0.0, 0.0)
    }

    fun min(): Point = Point(x, y)
    fun max(): Point = Point(x + width, y + height)
    fun center(): Point = Point(x + width / 2.0, y + height / 2.0)

    fun isEmpty(): Boolean = width <= 0.0 || height <= 0.0

    fun containsPoint(point: Point): Boolean =
        x <= point.x && point.x < x + width && y <= point.y && point.y < y + height

    /** An empty box is an identity, not a point to include in the envelope. */
    fun union(other: Rect): Rect {
        if (isEmpty()) return other
        if (other.isEmpty()) return this
        val left = min(x, other.x)
        val top = min(y, other.y)
        val right = max(x + width, other.x + other.width)
        val bottom = max(y + height, other.y + other.height)
        return Rect(left, top, right - left, bottom - top)
    }

    /** A boundary touch has no area and therefore no intersection value. */
    fun intersection(other: Rect): Rect? {
        val left = max(x, other.x)
        val top = max(y, other.y)
        val overlapWidth = min(x + width, other.x + other.width) - left
        val overlapHeight = min(y + height, other.y + other.height) - top
        if (overlapWidth <= 0.0 || overlapHeight <= 0.0) return null
        return Rect(left, top, overlapWidth, overlapHeight)
    }

    fun expandBy(amount: Double): Rect =
        Rect(x - amount, y - amount, width + 2 * amount, height + 2 * amount)
}
