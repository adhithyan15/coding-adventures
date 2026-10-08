package com.codingadventures.affine2d

import com.codingadventures.point2d.Point
import com.codingadventures.trig.Trig
import kotlin.math.abs

/**
 * Immutable six-scalar affine matrix in SVG order (G2D01).
 *
 * The hidden row [0,0,1] turns a position into [x,y,1] and a direction into
 * [x,y,0]. Translation therefore acts on positions, never directions.
 */
data class Affine2D(
    val a: Double, val b: Double, val c: Double,
    val d: Double, val e: Double, val f: Double,
) {
    companion object {
        fun identity() = Affine2D(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        fun translate(tx: Double, ty: Double) = Affine2D(1.0, 0.0, 0.0, 1.0, tx, ty)

        fun rotate(angle: Double): Affine2D {
            val sine = Trig.sin(angle)
            val cosine = Trig.cos(angle)
            return Affine2D(cosine, sine, -sine, cosine, 0.0, 0.0)
        }

        /** The center remains fixed after translation-rotation-translation. */
        fun rotateAround(center: Point, angle: Double) =
            translate(center.x, center.y).multiply(rotate(angle))
                .multiply(translate(-center.x, -center.y))

        fun scale(sx: Double, sy: Double) = Affine2D(sx, 0.0, 0.0, sy, 0.0, 0.0)
        fun scaleUniform(factor: Double) = scale(factor, factor)
        fun skewX(angle: Double) = Affine2D(1.0, 0.0, Trig.tan(angle), 1.0, 0.0, 0.0)
        fun skewY(angle: Double) = Affine2D(1.0, Trig.tan(angle), 0.0, 1.0, 0.0, 0.0)
    }

    /** The right operand acts first: [other] is applied before this matrix. */
    fun multiply(other: Affine2D) = Affine2D(
        a * other.a + c * other.b,
        b * other.a + d * other.b,
        a * other.c + c * other.d,
        b * other.c + d * other.d,
        a * other.e + c * other.f + e,
        b * other.e + d * other.f + f,
    )

    fun applyToPoint(point: Point) = Point(a * point.x + c * point.y + e,
        b * point.x + d * point.y + f)

    fun applyToVector(vector: Point) = Point(a * vector.x + c * vector.y,
        b * vector.x + d * vector.y)

    fun determinant(): Double = a * d - b * c

    /** Null means the area scale is too close to zero for a stable inverse. */
    fun invert(): Affine2D? {
        val det = determinant()
        if (abs(det) < 1e-12) return null
        val reciprocal = 1.0 / det
        return Affine2D(d * reciprocal, -b * reciprocal, -c * reciprocal,
            a * reciprocal, (c * f - d * e) * reciprocal,
            (b * e - a * f) * reciprocal)
    }

    fun isIdentity(): Boolean = abs(a - 1) < 1e-10 && abs(b) < 1e-10 &&
        abs(c) < 1e-10 && abs(d - 1) < 1e-10 && abs(e) < 1e-10 && abs(f) < 1e-10

    fun isTranslationOnly(): Boolean = abs(a - 1) < 1e-10 && abs(b) < 1e-10 &&
        abs(c) < 1e-10 && abs(d - 1) < 1e-10

    fun toArray(): DoubleArray = doubleArrayOf(a, b, c, d, e, f)
}
