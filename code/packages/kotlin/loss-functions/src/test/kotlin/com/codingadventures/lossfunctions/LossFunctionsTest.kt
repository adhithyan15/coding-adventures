package com.codingadventures.lossfunctions

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class LossFunctionsTest {

    private val tol = 1e-7

    // ========================================================================
    // MSE Tests
    // ========================================================================

    @Test
    fun `MSE parity vector`() {
        val result = LossFunctions.mse(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.9, 0.1, 0.2))
        assertEquals(0.02, result, tol)
    }

    @Test
    fun `MSE perfect prediction`() {
        val result = LossFunctions.mse(
            doubleArrayOf(1.0, 2.0, 3.0),
            doubleArrayOf(1.0, 2.0, 3.0))
        assertEquals(0.0, result, tol)
    }

    @Test
    fun `MSE symmetry`() {
        val a = doubleArrayOf(1.0, 2.0, 3.0)
        val b = doubleArrayOf(1.5, 2.5, 3.5)
        assertEquals(LossFunctions.mse(a, b), LossFunctions.mse(b, a), tol)
    }

    @Test
    fun `MSE derivative`() {
        val grad = LossFunctions.mseDerivative(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.9, 0.1, 0.2))
        val expected = doubleArrayOf(-2.0/30.0, 2.0/30.0, 4.0/30.0)
        for (i in grad.indices) {
            assertEquals(expected[i], grad[i], tol)
        }
    }

    @Test
    fun `MSE derivative sign convention`() {
        val grad = LossFunctions.mseDerivative(doubleArrayOf(0.0), doubleArrayOf(1.0))
        assertTrue(grad[0] > 0.0)
    }

    // ========================================================================
    // MAE Tests
    // ========================================================================

    @Test
    fun `MAE parity vector`() {
        val result = LossFunctions.mae(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.9, 0.1, 0.2))
        assertEquals(0.1333333333, result, tol)
    }

    @Test
    fun `MAE perfect prediction`() {
        val result = LossFunctions.mae(
            doubleArrayOf(1.0, 2.0, 3.0),
            doubleArrayOf(1.0, 2.0, 3.0))
        assertEquals(0.0, result, tol)
    }

    @Test
    fun `MAE derivative`() {
        val grad = LossFunctions.maeDerivative(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.9, 0.1, 0.2))
        val expected = doubleArrayOf(-1.0/3.0, 1.0/3.0, 1.0/3.0)
        for (i in grad.indices) {
            assertEquals(expected[i], grad[i], tol)
        }
    }

    @Test
    fun `MAE derivative at zero`() {
        val grad = LossFunctions.maeDerivative(doubleArrayOf(1.0), doubleArrayOf(1.0))
        assertEquals(0.0, grad[0], tol)
    }

    // ========================================================================
    // BCE Tests
    // ========================================================================

    @Test
    fun `BCE parity vector`() {
        val result = LossFunctions.bce(
            doubleArrayOf(1.0, 0.0, 1.0),
            doubleArrayOf(0.9, 0.1, 0.8))
        assertEquals(0.1446215275, result, 1e-6)
    }

    @Test
    fun `BCE non-negative`() {
        val result = LossFunctions.bce(
            doubleArrayOf(1.0, 0.0, 1.0),
            doubleArrayOf(0.5, 0.5, 0.5))
        assertTrue(result >= 0.0)
    }

    @Test
    fun `BCE derivative finite`() {
        val grad = LossFunctions.bceDerivative(
            doubleArrayOf(1.0, 0.0),
            doubleArrayOf(0.9, 0.1))
        for (g in grad) {
            assertFalse(g.isNaN())
            assertFalse(g.isInfinite())
        }
    }

    @Test
    fun `BCE handles edge predictions`() {
        val result = LossFunctions.bce(doubleArrayOf(1.0, 0.0), doubleArrayOf(0.0, 1.0))
        assertFalse(result.isNaN())
        assertFalse(result.isInfinite())
    }

    // ========================================================================
    // CCE Tests
    // ========================================================================

    @Test
    fun `CCE parity vector`() {
        val result = LossFunctions.cce(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.8, 0.1, 0.1))
        assertEquals(0.07438118, result, 1e-5)
    }

    @Test
    fun `CCE only correct class contributes`() {
        val loss1 = LossFunctions.cce(
            doubleArrayOf(1.0, 0.0, 0.0), doubleArrayOf(0.8, 0.1, 0.1))
        val loss2 = LossFunctions.cce(
            doubleArrayOf(1.0, 0.0, 0.0), doubleArrayOf(0.8, 0.05, 0.15))
        assertEquals(loss1, loss2, tol)
    }

    @Test
    fun `CCE non-negative`() {
        val result = LossFunctions.cce(
            doubleArrayOf(0.0, 1.0, 0.0),
            doubleArrayOf(0.1, 0.7, 0.2))
        assertTrue(result >= 0.0)
    }

    @Test
    fun `CCE derivative`() {
        val grad = LossFunctions.cceDerivative(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.8, 0.1, 0.1))
        assertTrue(grad[0] < 0.0)
        assertEquals(0.0, grad[1], 1e-5)
        assertEquals(0.0, grad[2], 1e-5)
    }

    @Test
    fun `CCE handles zero predictions`() {
        val result = LossFunctions.cce(doubleArrayOf(1.0, 0.0), doubleArrayOf(0.0, 1.0))
        assertFalse(result.isNaN())
        assertFalse(result.isInfinite())
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    @Test
    fun `empty array throws`() {
        assertThrows<IllegalArgumentException> {
            LossFunctions.mse(doubleArrayOf(), doubleArrayOf())
        }
    }

    @Test
    fun `mismatched length throws`() {
        assertThrows<IllegalArgumentException> {
            LossFunctions.mse(doubleArrayOf(1.0), doubleArrayOf(1.0, 2.0))
        }
    }

    // ========================================================================
    // Single Element Tests
    // ========================================================================

    @Test
    fun `single element MSE`() {
        assertEquals(0.25, LossFunctions.mse(doubleArrayOf(1.0), doubleArrayOf(0.5)), tol)
    }

    @Test
    fun `single element MAE`() {
        assertEquals(0.5, LossFunctions.mae(doubleArrayOf(1.0), doubleArrayOf(0.5)), tol)
    }
}
