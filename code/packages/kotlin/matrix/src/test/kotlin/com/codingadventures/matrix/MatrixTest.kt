package com.codingadventures.matrix

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class MatrixTest {

    private val eps = 1e-10

    // ========================================================================
    // Initialization
    // ========================================================================

    @Test fun `from scalar`() {
        val m = Matrix.fromScalar(5.0)
        assertEquals(1, m.rows); assertEquals(1, m.cols); assertEquals(5.0, m[0, 0])
    }

    @Test fun `from array`() {
        val m = Matrix.fromArray(doubleArrayOf(1.0, 2.0, 3.0))
        assertEquals(1, m.rows); assertEquals(3, m.cols); assertEquals(2.0, m[0, 1])
    }

    @Test fun `from 2D array`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(2, m.rows); assertEquals(2, m.cols); assertEquals(3.0, m[1, 0])
    }

    @Test fun `zeros`() {
        val m = Matrix.zeros(3, 2)
        assertEquals(3, m.rows); assertEquals(2, m.cols)
        for (i in 0 until 3) for (j in 0 until 2) assertEquals(0.0, m[i, j])
    }

    @Test fun `ragged array throws`() {
        assertThrows<IllegalArgumentException> {
            Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0)))
        }
    }

    // ========================================================================
    // Addition
    // ========================================================================

    @Test fun `add matrices`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        val b = Matrix.of(arrayOf(doubleArrayOf(5.0, 6.0), doubleArrayOf(7.0, 8.0)))
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(6.0, 8.0), doubleArrayOf(10.0, 12.0))), a + b)
    }

    @Test fun `add scalar`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(11.0, 12.0), doubleArrayOf(13.0, 14.0))), m + 10.0)
    }

    // ========================================================================
    // Subtraction
    // ========================================================================

    @Test fun `subtract matrices`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(5.0, 6.0), doubleArrayOf(7.0, 8.0)))
        val b = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(4.0, 4.0), doubleArrayOf(4.0, 4.0))), a - b)
    }

    @Test fun `subtract self`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(Matrix.zeros(2, 2), a - a)
    }

    // ========================================================================
    // Scale
    // ========================================================================

    @Test fun `scale`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(2.0, 4.0), doubleArrayOf(6.0, 8.0))), m * 2.0)
    }

    @Test fun `scale by zero`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0)))
        assertEquals(Matrix.zeros(1, 2), m * 0.0)
    }

    // ========================================================================
    // Transpose
    // ========================================================================

    @Test fun `transpose rectangular`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0, 3.0), doubleArrayOf(4.0, 5.0, 6.0)))
        val t = m.transpose()
        assertEquals(3, t.rows); assertEquals(2, t.cols)
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(1.0, 4.0), doubleArrayOf(2.0, 5.0), doubleArrayOf(3.0, 6.0))), t)
    }

    @Test fun `double transpose`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0, 3.0), doubleArrayOf(4.0, 5.0, 6.0)))
        assertEquals(m, m.transpose().transpose())
    }

    // ========================================================================
    // Dot Product
    // ========================================================================

    @Test fun `dot 2x2`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        val b = Matrix.of(arrayOf(doubleArrayOf(5.0, 6.0), doubleArrayOf(7.0, 8.0)))
        assertEquals(Matrix.of(arrayOf(doubleArrayOf(19.0, 22.0), doubleArrayOf(43.0, 50.0))), a.dot(b))
    }

    @Test fun `dot non-square`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0, 3.0)))
        val b = Matrix.of(arrayOf(doubleArrayOf(4.0), doubleArrayOf(5.0), doubleArrayOf(6.0)))
        val c = a.dot(b)
        assertEquals(1, c.rows); assertEquals(1, c.cols)
        assertEquals(32.0, c[0, 0], eps)
    }

    @Test fun `dot identity`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        val eye = Matrix.of(arrayOf(doubleArrayOf(1.0, 0.0), doubleArrayOf(0.0, 1.0)))
        assertEquals(a, a.dot(eye))
        assertEquals(a, eye.dot(a))
    }

    @Test fun `dot dimension mismatch`() {
        assertThrows<IllegalArgumentException> {
            Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0))).dot(Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0))))
        }
    }

    // ========================================================================
    // Equality & Immutability
    // ========================================================================

    @Test fun `equality`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        val b = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        assertEquals(a, b)
    }

    @Test fun `inequality`() {
        assertNotEquals(
            Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0))),
            Matrix.of(arrayOf(doubleArrayOf(1.0, 3.0))))
    }

    @Test fun `immutability`() {
        val a = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0), doubleArrayOf(3.0, 4.0)))
        a + Matrix.of(arrayOf(doubleArrayOf(10.0, 20.0), doubleArrayOf(30.0, 40.0)))
        assertEquals(1.0, a[0, 0])
    }

    @Test fun `getData returns copy`() {
        val m = Matrix.of(arrayOf(doubleArrayOf(1.0, 2.0)))
        val copy = m.getData()
        copy[0][0] = 999.0
        assertEquals(1.0, m[0, 0])
    }
}
